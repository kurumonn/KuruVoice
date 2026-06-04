//! プリセット → 各 DSP パラメータへの展開。設計書 4.4 / 5.9。
//!
//! プリセットは「声・EQ・コンプ・リミッター・ノイズゲート」のパラメータ集合として
//! `AppConfig` を生成する。`audio` セクション（デバイス等）は呼び出し側の現状値を
//! 維持したいので、`apply` では `audio` を上書きしない API も用意する。

use super::VoicePreset;
use crate::config::*;

/// プリセットから設定値を読み込むユーティリティ。設計書 5.9.2。
pub struct PresetManager;

impl PresetManager {
    /// プリセットに対応する `AppConfig` を生成する（audio はデフォルト）。
    pub fn load(preset: VoicePreset) -> AppConfig {
        let mut cfg = AppConfig::default();
        cfg.app.preset = preset.key().to_string();
        Self::apply(&mut cfg, preset);
        cfg
    }

    /// 既存設定の audio セクションなどを維持したまま、声づくり系パラメータだけを
    /// プリセット値で上書きする。GUI でプリセットボタンを押したときに使う。
    pub fn apply(cfg: &mut AppConfig, preset: VoicePreset) {
        cfg.app.preset = preset.key().to_string();
        match preset {
            VoicePreset::NeutralClean => {
                cfg.voice.pitch_semitones = 2.5;
                cfg.voice.formant_shift = 0.7;
                cfg.compressor = comp(-20.0, 2.5, 6.0, 100.0, 2.5);
                cfg.eq = eq(100.0, -0.5, -2.5, 2.5, -2.5);
                cfg.deesser = deesser(true, 6100.0, -29.0, 2.5, 6.0);
                cfg.harmonic = harmonic(0.25, 0.6, 0.5);
                cfg.noise_gate = gate(true, -46.0);
                cfg.denoise.enabled = true;
                cfg.denoise.amount = 0.35;
                cfg.limiter = limiter(-1.2, 45.0);
            }
            VoicePreset::SoftFeminine => {
                cfg.voice.pitch_semitones = 5.0;
                cfg.voice.formant_shift = 1.2;
                cfg.compressor = comp(-19.0, 2.8, 5.0, 95.0, 2.5);
                cfg.eq = eq(115.0, -1.5, -3.0, 2.8, -3.0);
                cfg.deesser = deesser(true, 5900.0, -31.0, 3.5, 9.0);
                cfg.harmonic = harmonic(0.32, 0.7, 0.5);
                cfg.noise_gate = gate(true, -47.0);
                cfg.denoise.enabled = true;
                cfg.denoise.amount = 0.40;
                cfg.limiter = limiter(-1.5, 42.0);
            }
            VoicePreset::BrightFeminine => {
                cfg.voice.pitch_semitones = 7.0;
                cfg.voice.formant_shift = 1.6;
                cfg.compressor = comp(-20.0, 2.6, 4.0, 85.0, 2.0);
                cfg.eq = eq(130.0, -2.0, -3.5, 3.6, -3.5);
                cfg.deesser = deesser(true, 5700.0, -33.0, 4.0, 11.0);
                cfg.harmonic = harmonic(0.40, 0.7, 0.6);
                cfg.noise_gate = gate(true, -48.0);
                cfg.denoise.enabled = true;
                cfg.denoise.amount = 0.45;
                cfg.limiter = limiter(-1.8, 38.0);
            }
            VoicePreset::YoungNeutral => {
                cfg.voice.pitch_semitones = 4.0;
                cfg.voice.formant_shift = 0.9;
                cfg.compressor = comp(-20.0, 2.4, 5.0, 95.0, 2.0);
                cfg.eq = eq(110.0, -1.0, -2.5, 2.6, -2.8);
                cfg.deesser = deesser(true, 6000.0, -30.0, 3.0, 8.0);
                cfg.harmonic = harmonic(0.28, 0.65, 0.5);
                cfg.noise_gate = gate(true, -47.0);
                cfg.denoise.enabled = true;
                cfg.denoise.amount = 0.38;
                cfg.limiter = limiter(-1.5, 42.0);
            }
            VoicePreset::NaturalLow => {
                cfg.voice.pitch_semitones = -2.0;
                cfg.voice.formant_shift = -0.5;
                cfg.compressor = comp(-20.0, 2.0, 12.0, 150.0, 2.0); // weak
                cfg.eq = eq(80.0, 0.0, -1.0, 1.0, -2.0); // natural
                cfg.deesser = deesser(true, 6500.0, -27.0, 2.0, 5.0);
                cfg.noise_gate = gate(true, -48.0);
                cfg.limiter = limiter(-1.0, 60.0);
            }
            VoicePreset::IkemenSoft => {
                cfg.voice.pitch_semitones = -3.0;
                cfg.voice.formant_shift = -0.8;
                cfg.compressor = comp(-18.0, 3.0, 8.0, 120.0, 3.0); // medium
                                                                    // presence +1.5dB, mud_cut -2.0dB
                cfg.eq = eq(80.0, 1.5, -2.0, 1.5, -2.5);
                cfg.deesser = deesser(true, 6400.0, -28.0, 2.4, 6.0);
                cfg.noise_gate = gate(true, -45.0);
                cfg.limiter = limiter(-1.0, 50.0);
            }
            VoicePreset::IkemenDeep => {
                cfg.voice.pitch_semitones = -4.0;
                cfg.voice.formant_shift = -1.2;
                cfg.compressor = comp(-18.0, 3.0, 8.0, 120.0, 3.0); // medium
                                                                    // low_boost +2.0dB, presence +1.0dB
                cfg.eq = eq(75.0, 2.0, -2.0, 1.0, -2.0);
                cfg.deesser = deesser(true, 6600.0, -27.0, 2.0, 5.0);
                cfg.noise_gate = gate(true, -45.0);
                cfg.limiter = limiter(-1.0, 60.0);
            }
            VoicePreset::Narrator => {
                cfg.voice.pitch_semitones = -2.0;
                cfg.voice.formant_shift = -0.6;
                cfg.compressor = comp(-16.0, 4.0, 6.0, 100.0, 4.0); // strong
                                                                    // presence +2.5dB
                cfg.eq = eq(90.0, 0.5, -1.5, 2.5, -3.0);
                cfg.deesser = deesser(true, 6200.0, -29.0, 3.0, 7.0);
                cfg.noise_gate = gate(true, -45.0);
                cfg.limiter = limiter(-1.5, 40.0); // strict
            }
            VoicePreset::ClearStreaming => {
                cfg.voice.pitch_semitones = -1.0;
                cfg.voice.formant_shift = -0.3;
                cfg.compressor = comp(-18.0, 3.0, 8.0, 120.0, 3.0);
                // presence +3.0dB, de_esser enabled
                cfg.eq = eq(90.0, 0.0, -1.0, 3.0, -3.5);
                cfg.deesser = deesser(true, 6000.0, -30.0, 3.2, 8.0);
                cfg.noise_gate = gate(true, -40.0); // medium
                cfg.limiter = limiter(-1.0, 50.0);
            }
            VoicePreset::RadioVoice => {
                cfg.voice.pitch_semitones = -3.0;
                cfg.voice.formant_shift = -1.0;
                cfg.compressor = comp(-16.0, 4.0, 6.0, 100.0, 5.0); // strong
                                                                    // low_boost +3.0dB, high_cut mild (de_esser として高域を弱める)
                cfg.eq = eq(70.0, 3.0, -1.5, 0.5, -4.0);
                cfg.deesser = deesser(true, 6700.0, -26.0, 2.0, 5.0);
                cfg.noise_gate = gate(true, -45.0);
                cfg.limiter = limiter(-1.0, 60.0);
            }
            VoicePreset::BrightHigh => {
                // 高音キャラ方向: 高く・細く・明るく（高品質ピッチ/フォルマントで自然に）
                cfg.voice.pitch_semitones = 6.0;
                cfg.voice.formant_shift = 2.0;
                cfg.compressor = comp(-18.0, 3.0, 8.0, 120.0, 3.0);
                cfg.eq = eq(120.0, -1.0, -2.0, 3.5, -3.0);
                cfg.deesser = deesser(true, 5800.0, -32.0, 3.6, 10.0);
                cfg.noise_gate = gate(true, -45.0);
                cfg.limiter = limiter(-1.0, 50.0);
            }
            VoicePreset::DeepCool => {
                // 低音キャラ方向: 低く・太く・渋く
                cfg.voice.pitch_semitones = -6.0;
                cfg.voice.formant_shift = -2.0;
                cfg.compressor = comp(-16.0, 4.0, 6.0, 110.0, 4.0); // strong
                cfg.eq = eq(70.0, 3.0, -1.5, 1.0, -2.0);
                cfg.deesser = deesser(true, 6800.0, -26.0, 2.0, 5.0);
                cfg.noise_gate = gate(true, -45.0);
                cfg.limiter = limiter(-1.0, 60.0);
            }
        }
    }
}

fn comp(
    threshold_db: f32,
    ratio: f32,
    attack_ms: f32,
    release_ms: f32,
    makeup_gain_db: f32,
) -> CompressorSection {
    CompressorSection {
        enabled: true,
        threshold_db,
        ratio,
        attack_ms,
        release_ms,
        makeup_gain_db,
    }
}

fn eq(
    high_pass_hz: f32,
    low_boost_db: f32,
    mud_cut_db: f32,
    presence_boost_db: f32,
    de_esser_db: f32,
) -> EqSection {
    EqSection {
        enabled: true,
        high_pass_hz,
        low_boost_db,
        mud_cut_db,
        presence_boost_db,
        de_esser_db,
    }
}

fn gate(enabled: bool, threshold_db: f32) -> NoiseGateSection {
    NoiseGateSection {
        enabled,
        threshold_db,
        attack_ms: 5.0,
        release_ms: 80.0,
    }
}

fn deesser(
    enabled: bool,
    frequency_hz: f32,
    threshold_db: f32,
    ratio: f32,
    max_reduction_db: f32,
) -> DeEsserSection {
    DeEsserSection {
        enabled,
        frequency_hz,
        threshold_db,
        ratio,
        max_reduction_db,
        attack_ms: 2.0,
        release_ms: 55.0,
    }
}

fn limiter(ceiling_db: f32, release_ms: f32) -> LimiterSection {
    LimiterSection {
        enabled: true,
        ceiling_db,
        release_ms,
    }
}

fn harmonic(amount: f32, warmth: f32, brightness: f32) -> HarmonicSection {
    HarmonicSection {
        enabled: true,
        amount,
        warmth,
        brightness,
    }
}
