//! Phase 0: 既存の仮想オーディオケーブル検出と配信ルーティング誘導。
//!
//! 自前の仮想ドライバを実装するまでの現実解として（docs/virtual_audio_design.md §4）、
//! VB-CABLE / VoiceMeeter / BlackHole などの既存仮想デバイスを出力一覧から検出し、
//! 「KuruVoice の出力に選ぶデバイス」と「受け側アプリで選ぶマイク名」を提示する。
//!
//! これにより、ユーザーは KuruVoice の GUI のワンクリックで配信用ルーティングを
//! 設定でき、体験を「ほぼ単体」に近づけられる。

use super::device;

/// 検出された仮想オーディオケーブル。
#[derive(Debug, Clone, PartialEq)]
pub struct VirtualCable {
    /// 種別表示名（"VB-CABLE" など）。
    pub kind: String,
    /// KuruVoice の出力に選ぶレンダーデバイス名。
    pub output_device: String,
    /// 受け側アプリ（OBS/Discord/VRChat）でマイクとして選ぶキャプチャ名の推定。
    pub mic_hint: String,
}

/// 現在の出力デバイス一覧から既知の仮想ケーブルを検出する。
pub fn detect() -> Vec<VirtualCable> {
    detect_from(&device::output_device_names())
}

/// 出力デバイス名のリストから仮想ケーブルを抽出する（テスト用に分離）。
pub fn detect_from(outputs: &[String]) -> Vec<VirtualCable> {
    outputs.iter().filter_map(|n| classify(n)).collect()
}

/// 出力デバイス名 1 件を分類する。仮想ケーブルでなければ None。
fn classify(output_name: &str) -> Option<VirtualCable> {
    let lower = output_name.to_lowercase();
    let kind = if lower.contains("voicemeeter") {
        "VoiceMeeter"
    } else if lower.contains("cable") {
        "VB-CABLE"
    } else if lower.contains("blackhole") {
        "BlackHole"
    } else {
        return None;
    };
    Some(VirtualCable {
        kind: kind.to_string(),
        output_device: output_name.to_string(),
        mic_hint: derive_mic_hint(output_name),
    })
}

/// 受け側で選ぶマイク名を推定する。
/// 多くの仮想ケーブルは "X Input"（レンダー）/ "X Output"（キャプチャ）の対で命名されるため、
/// 最初の "Input" を "Output" に置換する。入出力同名（BlackHole 等）はそのまま返す。
fn derive_mic_hint(output_name: &str) -> String {
    if output_name.contains("Input") {
        output_name.replacen("Input", "Output", 1)
    } else if output_name.contains("input") {
        output_name.replacen("input", "output", 1)
    } else {
        output_name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vbcable_input_to_output() {
        let c = classify("CABLE Input (VB-Audio Virtual Cable)").unwrap();
        assert_eq!(c.kind, "VB-CABLE");
        assert_eq!(c.mic_hint, "CABLE Output (VB-Audio Virtual Cable)");
    }

    #[test]
    fn voicemeeter_detected() {
        let c = classify("VoiceMeeter Input (VB-Audio VoiceMeeter VAIO)").unwrap();
        assert_eq!(c.kind, "VoiceMeeter");
        assert_eq!(c.mic_hint, "VoiceMeeter Output (VB-Audio VoiceMeeter VAIO)");
    }

    #[test]
    fn blackhole_same_name() {
        let c = classify("BlackHole 2ch").unwrap();
        assert_eq!(c.kind, "BlackHole");
        assert_eq!(c.mic_hint, "BlackHole 2ch");
    }

    #[test]
    fn normal_device_ignored() {
        assert!(classify("スピーカー (Realtek(R) Audio)").is_none());
        assert!(classify("Smart TV (NVIDIA High Definition Audio)").is_none());
    }

    #[test]
    fn detect_from_list() {
        let outputs = vec![
            "スピーカー (Realtek(R) Audio)".to_string(),
            "CABLE Input (VB-Audio Virtual Cable)".to_string(),
        ];
        let found = detect_from(&outputs);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].kind, "VB-CABLE");
    }
}
