//! 「声がどれくらい変わるか」を数値化するレポート。
//!
//! 合成音声（基音 130Hz + 倍音）を各プリセットに通し、加工前後を比較する:
//!   - 基本周波数 f0（自己相関で検出）→ ピッチが何半音下がったか
//!   - スペクトル重心 → 音色の明るさ/暗さ（太さ）の変化
//!   - 低域/高域エネルギー比 → 「太さ」の指標
//!   - RMS 音量変化
//!
//!   cargo run --release --example voice_report

use kuruvoice::config::AppConfig;
use kuruvoice::dsp::DspChain;
use kuruvoice::preset::{PresetManager, VoicePreset};
use rustfft::{num_complex::Complex, FftPlanner};
use std::f32::consts::TAU;

const SR: f32 = 48000.0;
const BLOCK: usize = 256;

/// 1/n ロールオフの倍音で声っぽい有声音を合成する。
fn synth_voice(seconds: f32, f0: f32) -> Vec<f32> {
    let n = (SR * seconds) as usize;
    let mut out = vec![0f32; n];
    for (i, sample) in out.iter_mut().enumerate() {
        let t = i as f32 / SR;
        let mut v = 0.0;
        let mut h = 1;
        loop {
            let f = f0 * h as f32;
            if f > SR / 2.0 || h > 60 {
                break;
            }
            v += (TAU * f * t).sin() / h as f32;
            h += 1;
        }
        *sample = v;
    }
    let peak = out.iter().fold(0f32, |m, &x| m.max(x.abs())).max(1e-9);
    for x in out.iter_mut() {
        *x = *x / peak * 0.3;
    }
    out
}

fn process(cfg: &AppConfig, input: &[f32]) -> Vec<f32> {
    let mut chain = DspChain::from_config(cfg, SR, BLOCK);
    let mut out = input.to_vec();
    for b in out.chunks_mut(BLOCK) {
        chain.process(b);
    }
    out
}

/// 自己相関による基本周波数検出（60〜400Hz を探索）。
fn detect_f0(x: &[f32]) -> f32 {
    let min_lag = (SR / 400.0) as usize;
    let max_lag = ((SR / 60.0) as usize).min(x.len() - 1);
    let mut best = 0f32;
    let mut best_lag = min_lag.max(1);
    for lag in min_lag..max_lag {
        let mut s = 0f32;
        for i in 0..(x.len() - lag) {
            s += x[i] * x[i + lag];
        }
        if s > best {
            best = s;
            best_lag = lag;
        }
    }
    SR / best_lag as f32
}

/// Hann 窓 + FFT で振幅スペクトル（0〜ナイキスト）を返す。
fn spectrum(x: &[f32]) -> Vec<f32> {
    let n = x.len();
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(n);
    let mut buf: Vec<Complex<f32>> = (0..n)
        .map(|i| {
            let w = 0.5 * (1.0 - (TAU * i as f32 / (n as f32 - 1.0)).cos());
            Complex {
                re: x[i] * w,
                im: 0.0,
            }
        })
        .collect();
    fft.process(&mut buf);
    buf[..n / 2].iter().map(|c| c.norm()).collect()
}

fn bin_hz(mag_len: usize) -> f32 {
    SR / (mag_len as f32 * 2.0)
}

fn centroid(mag: &[f32]) -> f32 {
    let bin = bin_hz(mag.len());
    let mut num = 0f32;
    let mut den = 0f32;
    for (i, &m) in mag.iter().enumerate() {
        num += i as f32 * bin * m;
        den += m;
    }
    if den <= 0.0 {
        0.0
    } else {
        num / den
    }
}

fn band_energy(mag: &[f32], lo: f32, hi: f32) -> f32 {
    let bin = bin_hz(mag.len());
    let mut s = 0f32;
    for (i, &m) in mag.iter().enumerate() {
        let f = i as f32 * bin;
        if f >= lo && f <= hi {
            s += m * m;
        }
    }
    s
}

fn rms_db(x: &[f32]) -> f32 {
    let ms: f32 = x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32;
    20.0 * ms.sqrt().max(1e-9).log10()
}

fn main() {
    let f0_in = 130.0_f32;
    let input = synth_voice(1.0, f0_in);

    let mid = input.len() / 2;
    let half = 8192; // 解析窓 16384 サンプル
    let seg = mid - half..mid + half;
    let pitch_seg = mid - 2048..mid + 2048;

    let in_f0 = detect_f0(&input[pitch_seg.clone()]);
    let in_mag = spectrum(&input[seg.clone()]);
    let in_centroid = centroid(&in_mag);
    let in_low = band_energy(&in_mag, 80.0, 300.0);
    let in_high = band_energy(&in_mag, 2000.0, 8000.0).max(1e-12);
    let in_ratio_db = 10.0 * (in_low / in_high).log10();
    let in_rms = rms_db(&input[seg.clone()]);

    println!("==== KuruVoice 声の変化量レポート ====");
    println!(
        "入力(合成音声): f0={:.1} Hz / スペクトル重心={:.0} Hz / 低域比={:+.1} dB",
        in_f0, in_centroid, in_ratio_db
    );
    println!();
    println!(
        "{:<16} {:>8} {:>9} {:>9} {:>9} {:>9} {:>9}",
        "preset", "f0(Hz)", "ピッチ半音", "重心Hz", "重心変化", "太さdB差", "音量dB"
    );

    let presets = [
        ("neutral_clean", VoicePreset::NeutralClean),
        ("soft_feminine", VoicePreset::SoftFeminine),
        ("bright_feminine", VoicePreset::BrightFeminine),
        ("young_neutral", VoicePreset::YoungNeutral),
        ("natural_low", VoicePreset::NaturalLow),
        ("ikemen_soft", VoicePreset::IkemenSoft),
        ("ikemen_deep", VoicePreset::IkemenDeep),
        ("narrator", VoicePreset::Narrator),
        ("clear_streaming", VoicePreset::ClearStreaming),
        ("radio_voice", VoicePreset::RadioVoice),
        ("bright_high", VoicePreset::BrightHigh),
        ("deep_cool", VoicePreset::DeepCool),
    ];

    for (name, p) in presets {
        let mut cfg = PresetManager::load(p);
        // この計測はピッチ/フォルマント/音色を測るもの。デノイザは「定常的な合成トーン」を
        // ノイズと誤判定して抑制してしまう（実音声＝非定常では起きにくい）ため、計測時は無効化。
        cfg.denoise.enabled = false;
        let out = process(&cfg, &input);

        let out_f0 = detect_f0(&out[pitch_seg.clone()]);
        let semitones = 12.0 * (out_f0 / in_f0).log2();
        let out_mag = spectrum(&out[seg.clone()]);
        let out_centroid = centroid(&out_mag);
        let cent_pct = (out_centroid - in_centroid) / in_centroid * 100.0;
        let out_low = band_energy(&out_mag, 80.0, 300.0);
        let out_high = band_energy(&out_mag, 2000.0, 8000.0).max(1e-12);
        let out_ratio_db = 10.0 * (out_low / out_high).log10();
        let out_rms = rms_db(&out[seg.clone()]);

        println!(
            "{:<16} {:>8.1} {:>+9.2} {:>9.0} {:>+8.1}% {:>+9.1} {:>+9.1}",
            name,
            out_f0,
            semitones,
            out_centroid,
            cent_pct,
            out_ratio_db - in_ratio_db,
            out_rms - in_rms
        );
    }

    println!();
    println!("ピッチ半音 : 負ほど低い声（プリセット設計値に一致するか）。");
    println!("重心Hz/変化: 小さいほど暗く太い、大きいほど明るく明瞭。");
    println!("太さdB差   : 低域(80-300Hz)/高域(2-8kHz) の比の変化。正で太く。");
}
