//! プリセット管理。設計書 5.9 / F-012。

mod presets;

pub use presets::PresetManager;

use std::str::FromStr;

/// 用途別ボイスプリセット。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoicePreset {
    NaturalLow,
    IkemenSoft,
    IkemenDeep,
    Narrator,
    ClearStreaming,
    RadioVoice,
}

impl VoicePreset {
    /// 全プリセットを定義順で返す（GUI のボタン生成などに使う）。
    pub fn all() -> [VoicePreset; 6] {
        [
            VoicePreset::NaturalLow,
            VoicePreset::IkemenSoft,
            VoicePreset::IkemenDeep,
            VoicePreset::Narrator,
            VoicePreset::ClearStreaming,
            VoicePreset::RadioVoice,
        ]
    }

    /// 設定ファイル等で使うスネークケースのキー。
    pub fn key(&self) -> &'static str {
        match self {
            VoicePreset::NaturalLow => "natural_low",
            VoicePreset::IkemenSoft => "ikemen_soft",
            VoicePreset::IkemenDeep => "ikemen_deep",
            VoicePreset::Narrator => "narrator",
            VoicePreset::ClearStreaming => "clear_streaming",
            VoicePreset::RadioVoice => "radio_voice",
        }
    }

    /// 画面表示用の名前。
    pub fn label(&self) -> &'static str {
        match self {
            VoicePreset::NaturalLow => "Natural Low",
            VoicePreset::IkemenSoft => "Ikemen Soft",
            VoicePreset::IkemenDeep => "Ikemen Deep",
            VoicePreset::Narrator => "Narrator",
            VoicePreset::ClearStreaming => "Clear Streaming",
            VoicePreset::RadioVoice => "Radio Voice",
        }
    }

    /// プリセットの短い説明。
    pub fn description(&self) -> &'static str {
        match self {
            VoicePreset::NaturalLow => "自然に少し低くする",
            VoicePreset::IkemenSoft => "爽やかで柔らかい低音",
            VoicePreset::IkemenDeep => "深めで落ち着いた声",
            VoicePreset::Narrator => "聞き取りやすいナレーション向け",
            VoicePreset::ClearStreaming => "配信向けに明瞭感重視",
            VoicePreset::RadioVoice => "ラジオ風の太い声",
        }
    }
}

impl FromStr for VoicePreset {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.trim().to_lowercase().replace([' ', '-'], "_").as_str() {
            "natural_low" => Ok(VoicePreset::NaturalLow),
            "ikemen_soft" => Ok(VoicePreset::IkemenSoft),
            "ikemen_deep" => Ok(VoicePreset::IkemenDeep),
            "narrator" => Ok(VoicePreset::Narrator),
            "clear_streaming" => Ok(VoicePreset::ClearStreaming),
            "radio_voice" => Ok(VoicePreset::RadioVoice),
            other => Err(format!("未知のプリセット: {other}")),
        }
    }
}
