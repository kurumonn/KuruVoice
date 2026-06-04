//! プリセット管理。設計書 5.9 / F-012。

mod presets;

pub use presets::PresetManager;

use std::str::FromStr;

/// 用途別ボイスプリセット。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoicePreset {
    NeutralClean,
    SoftFeminine,
    BrightFeminine,
    YoungNeutral,
    NaturalLow,
    IkemenSoft,
    IkemenDeep,
    Narrator,
    ClearStreaming,
    RadioVoice,
    BrightHigh,
    DeepCool,
}

impl VoicePreset {
    /// 全プリセットを定義順で返す（GUI のボタン生成などに使う）。
    pub fn all() -> [VoicePreset; 12] {
        [
            VoicePreset::NeutralClean,
            VoicePreset::SoftFeminine,
            VoicePreset::BrightFeminine,
            VoicePreset::YoungNeutral,
            VoicePreset::NaturalLow,
            VoicePreset::IkemenSoft,
            VoicePreset::IkemenDeep,
            VoicePreset::Narrator,
            VoicePreset::ClearStreaming,
            VoicePreset::RadioVoice,
            VoicePreset::BrightHigh,
            VoicePreset::DeepCool,
        ]
    }

    /// 設定ファイル等で使うスネークケースのキー。
    pub fn key(&self) -> &'static str {
        match self {
            VoicePreset::NeutralClean => "neutral_clean",
            VoicePreset::SoftFeminine => "soft_feminine",
            VoicePreset::BrightFeminine => "bright_feminine",
            VoicePreset::YoungNeutral => "young_neutral",
            VoicePreset::NaturalLow => "natural_low",
            VoicePreset::IkemenSoft => "ikemen_soft",
            VoicePreset::IkemenDeep => "ikemen_deep",
            VoicePreset::Narrator => "narrator",
            VoicePreset::ClearStreaming => "clear_streaming",
            VoicePreset::RadioVoice => "radio_voice",
            VoicePreset::BrightHigh => "bright_high",
            VoicePreset::DeepCool => "deep_cool",
        }
    }

    /// 画面表示用の名前。
    pub fn label(&self) -> &'static str {
        match self {
            VoicePreset::NeutralClean => "Neutral Clean",
            VoicePreset::SoftFeminine => "Soft Feminine",
            VoicePreset::BrightFeminine => "Bright Feminine",
            VoicePreset::YoungNeutral => "Young Neutral",
            VoicePreset::NaturalLow => "Natural Low",
            VoicePreset::IkemenSoft => "Ikemen Soft",
            VoicePreset::IkemenDeep => "Ikemen Deep",
            VoicePreset::Narrator => "Narrator",
            VoicePreset::ClearStreaming => "Clear Streaming",
            VoicePreset::RadioVoice => "Radio Voice",
            VoicePreset::BrightHigh => "Bright High",
            VoicePreset::DeepCool => "Deep Cool",
        }
    }

    /// プリセットの短い説明。
    pub fn description(&self) -> &'static str {
        match self {
            VoicePreset::NeutralClean => "性別感を薄めた自然で明瞭な声",
            VoicePreset::SoftFeminine => "落ち着いた女性寄りの柔らかい声",
            VoicePreset::BrightFeminine => "明るい女性寄りの高めの声",
            VoicePreset::YoungNeutral => "少年・中性寄りの軽い声",
            VoicePreset::NaturalLow => "自然に少し低くする",
            VoicePreset::IkemenSoft => "爽やかで柔らかい低音",
            VoicePreset::IkemenDeep => "深めで落ち着いた声",
            VoicePreset::Narrator => "聞き取りやすいナレーション向け",
            VoicePreset::ClearStreaming => "配信向けに明瞭感重視",
            VoicePreset::RadioVoice => "ラジオ風の太い声",
            VoicePreset::BrightHigh => "明るく軽い高めの声（高音キャラ方向）",
            VoicePreset::DeepCool => "低く渋い太めの声（低音キャラ方向）",
        }
    }
}

impl FromStr for VoicePreset {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.trim().to_lowercase().replace([' ', '-'], "_").as_str() {
            "neutral_clean" => Ok(VoicePreset::NeutralClean),
            "soft_feminine" => Ok(VoicePreset::SoftFeminine),
            "bright_feminine" => Ok(VoicePreset::BrightFeminine),
            "young_neutral" => Ok(VoicePreset::YoungNeutral),
            "natural_low" => Ok(VoicePreset::NaturalLow),
            "ikemen_soft" => Ok(VoicePreset::IkemenSoft),
            "ikemen_deep" => Ok(VoicePreset::IkemenDeep),
            "narrator" => Ok(VoicePreset::Narrator),
            "clear_streaming" => Ok(VoicePreset::ClearStreaming),
            "radio_voice" => Ok(VoicePreset::RadioVoice),
            "bright_high" => Ok(VoicePreset::BrightHigh),
            "deep_cool" => Ok(VoicePreset::DeepCool),
            other => Err(format!("未知のプリセット: {other}")),
        }
    }
}
