//! アプリケーション・オーケストレーション。
//! CLI / GUI 双方から使う「設定の解決」「デバイス一覧表示」「ヘッドレス実行」
//! 「テスト録音」をまとめる。

use crate::audio::{device, Engine};
use crate::cli::Args;
use crate::config::AppConfig;
use crate::dsp::meter::Meters;
use crate::dsp::DspChain;
use crate::preset::{PresetManager, VoicePreset};
use anyhow::{anyhow, Context, Result};
use cpal::traits::{DeviceTrait, StreamTrait};
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// 引数からアプリ設定を解決する。
/// 優先順位: 設定ファイル → プリセット指定で上書き → bypass 指定で上書き。
pub fn resolve_config(args: &Args) -> AppConfig {
    let mut cfg = match &args.config {
        Some(path) => AppConfig::load_or_default(path),
        None => AppConfig::default(),
    };

    if let Some(name) = &args.preset {
        match VoicePreset::from_str(name) {
            Ok(p) => PresetManager::apply(&mut cfg, p),
            Err(e) => log::warn!("{e}（設定値をそのまま使用します）"),
        }
    }

    if args.bypass {
        cfg.app.bypass = true;
    }

    cfg
}

/// `--list-devices`。設計書 F-003 / 5.10.2。
pub fn list_devices() {
    let (inputs, outputs) = device::collect_info();
    println!("=== 入力デバイス ===");
    if inputs.is_empty() {
        println!("  (見つかりません)");
    }
    for d in &inputs {
        let def = if d.is_default { " [default]" } else { "" };
        let sr = d
            .sample_rates
            .first()
            .map(|s| format!("{s}Hz"))
            .unwrap_or_else(|| "?".into());
        println!("  - {}{def}  (ch:{}, {sr})", d.name, d.channels);
    }
    println!("\n=== 出力デバイス ===");
    if outputs.is_empty() {
        println!("  (見つかりません)");
    }
    for d in &outputs {
        let def = if d.is_default { " [default]" } else { "" };
        let sr = d
            .sample_rates
            .first()
            .map(|s| format!("{s}Hz"))
            .unwrap_or_else(|| "?".into());
        println!("  - {}{def}  (ch:{}, {sr})", d.name, d.channels);
    }
}

/// ヘッドレス実行（GUI なし常駐）。Ctrl-C で終了。
pub fn run_headless(cfg: &AppConfig) -> Result<()> {
    let in_dev = device::find_input(&cfg.audio.input_device)
        .ok_or_else(|| anyhow!("入力デバイスが見つかりません"))?;
    let out_dev = device::find_output(&cfg.audio.output_device)
        .ok_or_else(|| anyhow!("出力デバイスが見つかりません"))?;

    let meters = Arc::new(Meters::default());
    let engine = Engine::start(&in_dev, &out_dev, cfg, meters.clone())
        .map_err(|e| anyhow!("エンジン起動失敗: {e}"))?;

    println!(
        "KuruVoice 実行中 ({}Hz, preset={}, bypass={})。Ctrl-C で終了。",
        engine.sample_rate, cfg.app.preset, cfg.app.bypass
    );

    let stop = Arc::new(AtomicBool::new(false));
    let stop_c = stop.clone();
    ctrlc_lite(move || stop_c.store(true, Ordering::Relaxed));

    while !stop.load(Ordering::Relaxed) {
        std::thread::sleep(std::time::Duration::from_millis(200));
        if log::log_enabled!(log::Level::Debug) {
            log::debug!("in:{:.3} out:{:.3}", meters.input(), meters.output());
        }
    }
    engine.stop();
    println!("\n終了しました。");
    Ok(())
}

/// `--record-test SECONDS`。設計書 F-017 / 5.10.2。
/// 生の入力を録音 → DSP チェーンでオフライン処理 → 2 つの WAV を書き出す。
pub fn record_test(cfg: &AppConfig, seconds: u32) -> Result<()> {
    let in_dev = device::find_input(&cfg.audio.input_device)
        .ok_or_else(|| anyhow!("入力デバイスが見つかりません"))?;
    let in_cfg = in_dev
        .default_input_config()
        .context("入力デフォルト設定取得失敗")?;
    let sample_rate = in_cfg.sample_rate().0;
    let channels = in_cfg.channels() as usize;

    println!("{seconds} 秒間のテスト録音を開始します... ({sample_rate}Hz)");

    let captured = Arc::new(std::sync::Mutex::new(Vec::<f32>::new()));
    let cap_cb = captured.clone();
    let stream = in_dev
        .build_input_stream(
            &cpal::StreamConfig {
                channels: in_cfg.channels(),
                sample_rate: in_cfg.sample_rate(),
                buffer_size: cpal::BufferSize::Default,
            },
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let mut g = cap_cb.lock().unwrap();
                for frame in data.chunks(channels.max(1)) {
                    let mono = frame.iter().sum::<f32>() / channels.max(1) as f32;
                    g.push(mono);
                }
            },
            |e| log::error!("録音ストリームエラー: {e}"),
            None,
        )
        .context("録音ストリーム構築失敗")?;
    stream.play().context("録音開始失敗")?;
    std::thread::sleep(std::time::Duration::from_secs(seconds as u64));
    drop(stream);

    let raw = captured.lock().unwrap().clone();
    println!("録音サンプル数: {}", raw.len());

    // オフライン処理
    let mut processed = raw.clone();
    let mut chain = DspChain::from_config(cfg, sample_rate as f32, 256);
    for block in processed.chunks_mut(256) {
        chain.process(block);
    }

    write_wav("kuruvoice_raw.wav", &raw, sample_rate)?;
    write_wav("kuruvoice_processed.wav", &processed, sample_rate)?;
    println!("書き出し: kuruvoice_raw.wav / kuruvoice_processed.wav");
    Ok(())
}

fn write_wav(path: &str, samples: &[f32], sample_rate: u32) -> Result<()> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut w = hound::WavWriter::create(path, spec).context("WAV 作成失敗")?;
    for &s in samples {
        w.write_sample(s).context("WAV 書き込み失敗")?;
    }
    w.finalize().context("WAV finalize 失敗")?;
    Ok(())
}

/// 依存を増やさない簡易 Ctrl-C ハンドラ（best-effort）。
fn ctrlc_lite<F: Fn() + Send + 'static>(f: F) {
    // 標準ライブラリだけでは移植性のある Ctrl-C フックがないため、
    // ここでは別スレッドで標準入力の EOF/改行を待つ簡易版とする。
    std::thread::spawn(move || {
        let mut line = String::new();
        let _ = std::io::stdin().read_line(&mut line);
        f();
    });
    println!("(Enter キーでも終了できます)");
}
