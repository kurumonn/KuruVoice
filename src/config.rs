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
    pub fluctuation: FluctuationSection,
    pub denoise: DenoiseSection,
    pub auto_gain: AutoGainSection,
    pub noise_gate: NoiseGateSection,
    pub eq: EqSection,
    pub harmonic: HarmonicSection,
    pub deesser: DeEsserSection,
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

/// 1/f ゆらぎモード。KV-DSP-5。
/// ピンクノイズ(1/f)で駆動する微小なピッチ揺れ(マイクロ・ビブラート)と音量揺れ(トレモロ)を
/// 加え、機械的・平坦な合成感を消して「人間らしい・心地よい」自然な声にする。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FluctuationSection {
    pub enabled: bool,
    /// 全体の効き 0.0〜1.0。
    pub amount: f32,
    /// ピッチ揺れの最大幅（セント）。
    pub pitch_cents: f32,
    /// 音量揺れの深さ 0.0〜1.0。
    pub amp_depth: f32,
    /// 揺らぎの基準レート(Hz)。小さいほどゆっくり。
    pub rate_hz: f32,
}
impl Default for FluctuationSection {
    fn default() -> Self {
        Self {
            enabled: false,
            amount: 0.5,
            pitch_cents: 12.0,
            amp_depth: 0.1,
            rate_hz: 5.0,
        }
    }
}

/// ノイズキャンセル（STFT スペクトル減算）。ノイズゲートとは別物で、
/// 発話中も含めて定常的な背景ノイズ（ファン・ホワイトノイズ等）を低減する。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DenoiseSection {
    pub enabled: bool,
    /// 低減強度 0.0〜1.0（大きいほど強く除去。かけすぎると不自然）。
    pub amount: f32,
}
impl Default for DenoiseSection {
    fn default() -> Self {
        Self {
            enabled: false,
            amount: 0.5,
        }
    }
}

/// オートゲイン（AGC）。KV-IN-2 / FR-002。小さい声・大きい声を目標レベルへ
/// 緩やかに均す。無音(ゲート以下)では利得を保持してノイズを増幅しない。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AutoGainSection {
    pub enabled: bool,
    /// 目標 RMS レベル(dBFS)。
    pub target_db: f32,
    /// 最大ブースト量(dB)。これ以上は持ち上げない（ノイズ増幅防止）。
    pub max_gain_db: f32,
    /// この入力レベル(dBFS)以下では利得を更新せず保持する（無音時のポンピング防止）。
    pub gate_db: f32,
}
impl Default for AutoGainSection {
    fn default() -> Self {
        Self {
            enabled: false,
            target_db: -20.0,
            max_gain_db: 18.0,
            gate_db: -50.0,
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

/// 動的 De-esser。EQ の静的ハイシェルフとは別に、サ行・歯擦音が強い瞬間だけ
/// 高域成分を抑える。女性声・高めプリセットの刺さり対策に使う。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DeEsserSection {
    pub enabled: bool,
    pub frequency_hz: f32,
    pub threshold_db: f32,
    pub ratio: f32,
    pub max_reduction_db: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
}
impl Default for DeEsserSection {
    fn default() -> Self {
        Self {
            enabled: true,
            frequency_hz: 6200.0,
            threshold_db: -28.0,
            ratio: 3.0,
            max_reduction_db: 8.0,
            attack_ms: 2.0,
            release_ms: 55.0,
        }
    }
}

/// ハーモニック・エンハンサー（倍音生成）。KV-DSP-1。
/// 上げた声（女性/中性）が「細い・芯がない」のを、低中域の偶数次倍音（太さ）と
/// 高域の奇数次倍音（艶/密度）を足して補強する。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HarmonicSection {
    pub enabled: bool,
    /// 効き 0.0〜1.0（大きいほど倍音を強く付加。かけすぎると歪む）。
    pub amount: f32,
    /// 「芯/太さ」（低中域の偶数次倍音）の比率 0.0〜1.0。
    pub warmth: f32,
    /// 「艶/密度」（高域の奇数次倍音）の比率 0.0〜1.0。
    pub brightness: f32,
}
impl Default for HarmonicSection {
    fn default() -> Self {
        Self {
            enabled: false,
            amount: 0.3,
            warmth: 0.6,
            brightness: 0.5,
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
