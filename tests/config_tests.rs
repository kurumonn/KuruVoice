//! 設定・プリセットのテスト。設計書 7.1 / 7.2。

use kuruvoice::config::AppConfig;
use kuruvoice::preset::{PresetManager, VoicePreset};
use std::str::FromStr;

#[test]
fn default_config_is_ikemen_soft() {
    let cfg = AppConfig::default();
    assert_eq!(cfg.app.preset, "ikemen_soft");
    assert_eq!(cfg.audio.sample_rate, 48000);
    assert!(cfg.limiter.enabled);
}

#[test]
fn toml_roundtrip() {
    // 7.1 設定ファイル読み込み + F-014 保存
    let cfg = AppConfig::default();
    let text = toml::to_string_pretty(&cfg).expect("serialize");
    let parsed: AppConfig = toml::from_str(&text).expect("deserialize");
    assert_eq!(cfg, parsed);
}

#[test]
fn partial_toml_uses_defaults() {
    // serde(default) で一部だけの TOML も読める
    let text = r#"
        [voice]
        pitch_semitones = -5.0
    "#;
    let cfg: AppConfig = toml::from_str(text).expect("parse partial");
    assert_eq!(cfg.voice.pitch_semitones, -5.0);
    // 他はデフォルト
    assert_eq!(cfg.audio.sample_rate, 48000);
    assert!(cfg.eq.enabled);
}

#[test]
fn preset_parsing() {
    assert_eq!(
        VoicePreset::from_str("ikemen_soft").unwrap(),
        VoicePreset::IkemenSoft
    );
    assert_eq!(
        VoicePreset::from_str("Ikemen Deep").unwrap(),
        VoicePreset::IkemenDeep
    );
    assert_eq!(
        VoicePreset::from_str("radio-voice").unwrap(),
        VoicePreset::RadioVoice
    );
    assert!(VoicePreset::from_str("unknown").is_err());
}

#[test]
fn preset_values_match_spec() {
    // 4.4 の代表値を確認
    let natural = PresetManager::load(VoicePreset::NaturalLow);
    assert_eq!(natural.voice.pitch_semitones, -2.0);

    let deep = PresetManager::load(VoicePreset::IkemenDeep);
    assert_eq!(deep.voice.pitch_semitones, -4.0);
    assert_eq!(deep.voice.formant_shift, -1.2);

    let clear = PresetManager::load(VoicePreset::ClearStreaming);
    assert_eq!(clear.voice.pitch_semitones, -1.0);
    assert!(clear.eq.presence_boost_db >= 3.0);
}

#[test]
fn apply_preset_keeps_audio_section() {
    // GUI でプリセット適用時、audio（デバイス等）は維持される
    let mut cfg = AppConfig::default();
    cfg.audio.input_device = "MyMic".to_string();
    cfg.audio.output_device = "VB-CABLE".to_string();
    PresetManager::apply(&mut cfg, VoicePreset::RadioVoice);
    assert_eq!(cfg.audio.input_device, "MyMic");
    assert_eq!(cfg.audio.output_device, "VB-CABLE");
    assert_eq!(cfg.app.preset, "radio_voice");
}
