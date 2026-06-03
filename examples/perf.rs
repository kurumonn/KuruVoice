//! E2E 負荷テスト（設計書 7.4 パフォーマンステスト）。
//!
//! 声に似た信号を 60 秒ぶん、DSP チェーンにブロック単位で通し、
//! 1 ブロックあたりの処理時間（平均 / p50 / p99 / 最大）と、
//! 「リアルタイムの何倍速で処理できたか」「1 コアでの CPU 負荷率」を計測する。
//!
//!   cargo run --release --example perf

use kuruvoice::config::AppConfig;
use kuruvoice::dsp::DspChain;
use kuruvoice::preset::{PresetManager, VoicePreset};
use std::f32::consts::TAU;
use std::time::Instant;

const SR: f32 = 48000.0;
const BLOCK: usize = 256;

/// 基音 + 倍音の声っぽい信号でブロックを満たす。
fn synth_block(buf: &mut [f32], phase: &mut f32, f0: f32) {
    for s in buf.iter_mut() {
        let t = *phase;
        let v = t.sin() * 0.5
            + (2.0 * t).sin() * 0.25
            + (3.0 * t).sin() * 0.12
            + (4.0 * t).sin() * 0.06;
        *s = v * 0.4;
        *phase += TAU * f0 / SR;
        if *phase > TAU {
            *phase -= TAU;
        }
    }
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((p / 100.0) * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx]
}

fn main() {
    let deadline_ms = BLOCK as f64 / SR as f64 * 1000.0;
    let audio_seconds = 60.0_f64;
    let n_blocks = (SR as f64 * audio_seconds / BLOCK as f64) as usize;

    println!("==== KuruVoice E2E 負荷テスト ====");
    println!(
        "サンプルレート {} Hz / ブロック {} サンプル / ブロック締切 {:.3} ms / 計測 {} 秒ぶん({} ブロック)",
        SR as u32, BLOCK, deadline_ms, audio_seconds as u32, n_blocks
    );
    println!();
    println!(
        "{:<22} {:>9} {:>9} {:>9} {:>9} {:>11} {:>11}",
        "preset", "avg(ms)", "p50(ms)", "p99(ms)", "max(ms)", "CPU%/1core", "realtime x"
    );

    let mut bypass_cfg = AppConfig::default();
    bypass_cfg.app.bypass = true;

    let cases: Vec<(&str, AppConfig)> = vec![
        ("full(ikemen_soft)", AppConfig::default()),
        ("ikemen_deep", PresetManager::load(VoicePreset::IkemenDeep)),
        ("radio_voice", PresetManager::load(VoicePreset::RadioVoice)),
        ("bypass", bypass_cfg),
    ];

    for (name, cfg) in cases {
        let mut chain = DspChain::from_config(&cfg, SR, BLOCK);
        let mut buf = vec![0f32; BLOCK];
        let mut phase = 0f32;

        // ウォームアップ（キャッシュ・フィルタ充填）
        for _ in 0..200 {
            synth_block(&mut buf, &mut phase, 130.0);
            chain.process(&mut buf);
        }

        let mut times = Vec::with_capacity(n_blocks);
        for _ in 0..n_blocks {
            synth_block(&mut buf, &mut phase, 130.0);
            let t0 = Instant::now();
            chain.process(&mut buf);
            times.push(t0.elapsed().as_secs_f64() * 1000.0);
        }

        let sum: f64 = times.iter().sum();
        let avg = sum / times.len() as f64;
        let mut sorted = times.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let max = *sorted.last().unwrap();
        let cpu = avg / deadline_ms * 100.0;
        let processed_audio_ms = n_blocks as f64 * deadline_ms;
        let realtime_factor = processed_audio_ms / sum;

        println!(
            "{:<22} {:>9.4} {:>9.4} {:>9.4} {:>9.4} {:>10.2}% {:>10.1}x",
            name,
            avg,
            percentile(&sorted, 50.0),
            percentile(&sorted, 99.0),
            max,
            cpu,
            realtime_factor
        );
    }

    println!();
    println!("CPU%/1core : 1コアでリアルタイムを維持するのに必要な割合（100%未満なら余裕あり）。");
    println!("realtime x : 実時間の何倍速で処理できたか（大きいほど低負荷）。");
    println!(
        "バッファ落ち : 各ブロックの max が締切 {:.2} ms 未満ならアンダーラン無し。",
        deadline_ms
    );
}
