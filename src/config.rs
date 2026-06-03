//! 設定ファイル (TOML) のスキーマと読み書き。
//!
//! 設計書 4.5 / F-013 / F-014 に対応。`#[serde(default)]` を全フィールドに付与し、
//! 一部だけ記述した TOML でも安全に読み込めるようにする。読み込み失敗時は
//! 呼び出し側でデフォルト設定にフォールバックする (ConfigLoadError → default)。

use crate::error::{KuruError, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// アプリ全体設定。TOML のトップレベル。
/// デフォルトは "Ikemen Soft" 相当（config.example.toml と一致）。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub app: AppSection,
    pub audio: AudioSection,
    pub voice: VoiceSection,
    pub noise_gate: NoiseGateSection,
    pub eq: EqSection,
    pub compressor: CompressorSection,
    pub limiter: LimiterSection,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSection {
    pub preset: String,
    pub bypass: bool,
}
impl Default for AppSection {
    fn default() -> Self {
        Self {
            preset: "ikemen_soft".to_string(),
            bypass: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioSection {
    pub input_device: String,
    pub output_device: String,
    pub sample_rate: u32,
    pub buffer_size: usize,
    pub channels: u16,
}
impl Default for AudioSection {
    fn default() -> Self {
        Self {
            input_device: "default".to_string(),
            output_device: "default".to_string(),
            sample_rate: 48000,
            buffer_size: 256,
            channels: 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VoiceSection {
    pub pitch_semitones: f32,
    pub formant_shift: f32,
    pub mix: f32,
}
impl Default for VoiceSection {
    fn default() -> Self {
        Self {
            pitch_semitones: -3.0,
            formant_shift: -0.8,
            mix: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct NoiseGateSection {
    pub enabled: bool,
    pub threshold_db: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
}
impl Default for NoiseGateSection {
    fn default() -> Self {
        Self {
            enabled: true,
            threshold_db: -45.0,
            attack_ms: 5.0,
            release_ms: 80.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EqSection {
    pub enabled: bool,
    pub high_pass_hz: f32,
    pub low_boost_db: f32,
    pub mud_cut_db: f32,
    pub presence_boost_db: f32,
    pub de_esser_db: f32,
}
impl Default for EqSection {
    fn default() -> Self {
        Self {
            enabled: true,
            high_pass_hz: 80.0,
            low_boost_db: 1.5,
            mud_cut_db: -2.0,
            presence_boost_db: 2.0,
            de_esser_db: -2.5,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CompressorSection {
    pub enabled: bool,
    pub threshold_db: f32,
    pub ratio: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
    pub makeup_gain_db: f32,
}
impl Default for CompressorSection {
    fn default() -> Self {
        Self {
            enabled: true,
            threshold_db: -18.0,
            ratio: 3.0,
            attack_ms: 8.0,
            release_ms: 120.0,
            makeup_gain_db: 3.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LimiterSection {
    pub enabled: bool,
    pub ceiling_db: f32,
    pub release_ms: f32,
}
impl Default for LimiterSection {
    fn default() -> Self {
        Self {
            enabled: true,
            ceiling_db: -1.0,
            release_ms: 50.0,
        }
    }
}

impl AppConfig {
    /// TOML ファイルを読み込む。F-013。
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let text = std::fs::read_to_string(&path)
            .map_err(|e| KuruError::ConfigLoadError(e.to_string()))?;
        let cfg: AppConfig =
            toml::from_str(&text).map_err(|e| KuruError::ConfigLoadError(e.to_string()))?;
        Ok(cfg)
    }

    /// 読み込みに失敗しても panic させず、デフォルト設定で復帰する。
    /// 設計書 6.1 ConfigLoadError → 「デフォルト設定で起動」。
    pub fn load_or_default<P: AsRef<Path>>(path: P) -> Self {
        match Self::load(&path) {
            Ok(cfg) => cfg,
            Err(e) => {
                log::warn!("設定ファイル読み込み失敗 ({e}). デフォルト設定で起動します。");
                Self::default()
            }
        }
    }

    /// 現在の設定を TOML として保存する。F-014。
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let text =
            toml::to_string_pretty(self).map_err(|e| KuruError::ConfigSaveError(e.to_string()))?;
        std::fs::write(&path, text).map_err(|e| KuruError::ConfigSaveError(e.to_string()))?;
        Ok(())
    }
}
