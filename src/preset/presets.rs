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
            VoicePreset::NaturalLow => {
                cfg.voice.pitch_semitones = -2.0;
                cfg.voice.formant_shift = -0.5;
                cfg.compressor = comp(-20.0, 2.0, 12.0, 150.0, 2.0); // weak
                cfg.eq = eq(80.0, 0.0, -1.0, 1.0, -2.0); // natural
                cfg.noise_gate = gate(true, -48.0);
                cfg.limiter = limiter(-1.0, 60.0);
            }
            VoicePreset::IkemenSoft => {
                cfg.voice.pitch_semitones = -3.0;
                cfg.voice.formant_shift = -0.8;
                cfg.compressor = comp(-18.0, 3.0, 8.0, 120.0, 3.0); // medium
                                                                    // presence +1.5dB, mud_cut -2.0dB
                cfg.eq = eq(80.0, 1.5, -2.0, 1.5, -2.5);
                cfg.noise_gate = gate(true, -45.0);
                cfg.limiter = limiter(-1.0, 50.0);
            }
            VoicePreset::IkemenDeep => {
                cfg.voice.pitch_semitones = -4.0;
                cfg.voice.formant_shift = -1.2;
                cfg.compressor = comp(-18.0, 3.0, 8.0, 120.0, 3.0); // medium
                                                                    // low_boost +2.0dB, presence +1.0dB
                cfg.eq = eq(75.0, 2.0, -2.0, 1.0, -2.0);
                cfg.noise_gate = gate(true, -45.0);
                cfg.limiter = limiter(-1.0, 60.0);
            }
            VoicePreset::Narrator => {
                cfg.voice.pitch_semitones = -2.0;
                cfg.voice.formant_shift = -0.6;
                cfg.compressor = comp(-16.0, 4.0, 6.0, 100.0, 4.0); // strong
                                                                    // presence +2.5dB
                cfg.eq = eq(90.0, 0.5, -1.5, 2.5, -3.0);
                cfg.noise_gate = gate(true, -45.0);
                cfg.limiter = limiter(-1.5, 40.0); // strict
            }
            VoicePreset::ClearStreaming => {
                cfg.voice.pitch_semitones = -1.0;
                cfg.voice.formant_shift = -0.3;
                cfg.compressor = comp(-18.0, 3.0, 8.0, 120.0, 3.0);
                // presence +3.0dB, de_esser enabled
                cfg.eq = eq(90.0, 0.0, -1.0, 3.0, -3.5);
                cfg.noise_gate = gate(true, -40.0); // medium
                cfg.limiter = limiter(-1.0, 50.0);
            }
            VoicePreset::RadioVoice => {
                cfg.voice.pitch_semitones = -3.0;
                cfg.voice.formant_shift = -1.0;
                cfg.compressor = comp(-16.0, 4.0, 6.0, 100.0, 5.0); // strong
                                                                    // low_boost +3.0dB, high_cut mild (de_esser として高域を弱める)
                cfg.eq = eq(70.0, 3.0, -1.5, 0.5, -4.0);
                cfg.noise_gate = gate(true, -45.0);
                cfg.limiter = limiter(-1.0, 60.0);
            }
            VoicePreset::BrightHigh => {
                // 高音キャラ方向: 高く・細く・明るく（高品質ピッチ/フォルマントで自然に）
                cfg.voice.pitch_semitones = 6.0;
                cfg.voice.formant_shift = 2.0;
                cfg.compressor = comp(-18.0, 3.0, 8.0, 120.0, 3.0);
                cfg.eq = eq(120.0, -1.0, -2.0, 3.5, -3.0);
                cfg.noise_gate = gate(true, -45.0);
                cfg.limiter = limiter(-1.0, 50.0);
            }
            VoicePreset::DeepCool => {
                // 低音キャラ方向: 低く・太く・渋く
                cfg.voice.pitch_semitones = -6.0;
                cfg.voice.formant_shift = -2.0;
                cfg.compressor = comp(-16.0, 4.0, 6.0, 110.0, 4.0); // strong
                cfg.eq = eq(70.0, 3.0, -1.5, 1.0, -2.0);
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

fn limiter(ceiling_db: f32, release_ms: f32) -> LimiterSection {
    LimiterSection {
        enabled: true,
        ceiling_db,
        release_ms,
    }
}
