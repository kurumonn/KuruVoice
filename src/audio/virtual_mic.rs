//! Phase 1: Linux ネイティブ仮想マイク（PipeWire / PulseAudio）。
//! docs/virtual_audio_design.md §3.1。
//!
//! `pactl` で null-sink「KuruVoice_Sink」と、その monitor を remap した
//! source「KuruVoice_Mic」を実行時に作成する。KuruVoice の出力を
//! `KuruVoice_Sink` へ向けると、他アプリ（OBS/Discord/VRChat）は
//! `KuruVoice_Mic` をマイクとして選べる。外部ソフトの追加は不要。
//!
//! 注意:
//! - Windows / macOS では未対応（`create()` がエラーを返す）。Windows は
//!   従来どおり Phase 0 の仮想ケーブル連携を使う。
//! - PipeWire でも `pactl`（pipewire-pulse 互換層）で動作する。
//! - 本コードは Linux 実機での動作確認が必要（cpal の出力先として
//!   `KuruVoice_Sink` が見えること等は環境に依存する）。

use anyhow::{bail, Context, Result};
use std::process::Command;

/// KuruVoice が出力する仮想シンク名。
pub const SINK_NAME: &str = "KuruVoice_Sink";
/// 受け側アプリがマイクとして選ぶ仮想ソース名。
pub const MIC_NAME: &str = "KuruVoice_Mic";

/// 実行時に作成した仮想マイク。Drop で自動的に破棄する。
#[derive(Debug)]
pub struct VirtualMic {
    sink_module: Option<u32>,
    source_module: Option<u32>,
}

impl VirtualMic {
    /// 現在の OS でネイティブ仮想マイクに対応しているか。
    pub fn is_supported() -> bool {
        cfg!(target_os = "linux")
    }

    /// 仮想シンクと仮想マイクを作成する。
    pub fn create() -> Result<Self> {
        if !Self::is_supported() {
            bail!(
                "ネイティブ仮想マイクは現在 Linux のみ対応です（Windows は仮想ケーブル連携を使用）"
            );
        }
        let sink_module = load_module(&[
            "module-null-sink".to_string(),
            format!("sink_name={SINK_NAME}"),
            format!("sink_properties=device.description={SINK_NAME}"),
        ])
        .context("null-sink の作成に失敗")?;

        let source_module = load_module(&[
            "module-remap-source".to_string(),
            format!("master={SINK_NAME}.monitor"),
            format!("source_name={MIC_NAME}"),
            format!("source_properties=device.description={MIC_NAME}"),
        ])
        .context("remap-source の作成に失敗")?;

        Ok(Self {
            sink_module: Some(sink_module),
            source_module: Some(source_module),
        })
    }

    pub fn sink_name(&self) -> &'static str {
        SINK_NAME
    }

    pub fn mic_name(&self) -> &'static str {
        MIC_NAME
    }

    /// 作成したモジュールを解放する（source → sink の順）。
    pub fn destroy(&mut self) {
        for id in [self.source_module.take(), self.sink_module.take()]
            .into_iter()
            .flatten()
        {
            let _ = Command::new("pactl")
                .arg("unload-module")
                .arg(id.to_string())
                .output();
        }
    }
}

impl Drop for VirtualMic {
    fn drop(&mut self) {
        self.destroy();
    }
}

/// `pactl load-module <args>` を実行し、返ってくるモジュール ID を返す。
fn load_module(args: &[String]) -> Result<u32> {
    let out = Command::new("pactl")
        .arg("load-module")
        .args(args)
        .output()
        .context("pactl の実行に失敗（PulseAudio / PipeWire が必要）")?;
    if !out.status.success() {
        bail!(
            "pactl load-module 失敗: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse::<u32>()
        .context("module id の解析に失敗")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_stable() {
        assert_eq!(SINK_NAME, "KuruVoice_Sink");
        assert_eq!(MIC_NAME, "KuruVoice_Mic");
    }

    #[test]
    fn unsupported_off_linux() {
        if !cfg!(target_os = "linux") {
            assert!(!VirtualMic::is_supported());
            assert!(VirtualMic::create().is_err());
        }
    }
}
