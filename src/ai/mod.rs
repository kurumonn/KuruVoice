//! AI 推論ブロック（T-016）。
//!
//! `ai` feature が有効なとき ONNX Runtime (ort) を使った推論を DSP チェーンに挿入できる。
//! 推論は専用スレッドで非同期実行し、PCM を crossbeam-channel でやり取りすることで
//! RT コールバックをブロックしない。
//!
//! 現在は AudioProcessor トレイトを実装した identity スタブのみ。
//! T-017 でモデルを学習し assets/models/voice_enhance_v1.onnx として同梱する予定。
//!
//! 安全方針:
//! - 特定人物の声紋への意図的な近似を拒否するスコアリングを将来実装する（設計書 safety.md 参照）。
//! - ユーザーが自分の声を整える目的以外での利用は禁止（LICENSE / docs/safety.md）。

use crate::config::AppConfig;
use crate::dsp::AudioProcessor;

/// ONNX モデルを使う推論ブロック。`ai` feature が無効なときでも型として存在するが
/// `prepare` は no-op、`process` は入力をそのまま通過させる（identity）。
pub struct OnnxInferenceBlock {
    enabled: bool,
    /// モデルファイルパス（None = 推論なし）。
    model_path: Option<String>,
    // 将来: ort::Session, 推論スレッドへの送信チャネル等をここに追加。
}

impl OnnxInferenceBlock {
    pub fn new(model_path: Option<String>) -> Self {
        Self {
            enabled: model_path.is_some(),
            model_path,
        }
    }

    /// `AppConfig.audio.ai_model_path` からブロックを構築する。
    pub fn from_config(_cfg: &AppConfig) -> Self {
        // T-017 完了後に cfg.audio.ai_model_path を参照する。
        Self::new(None)
    }
}

impl AudioProcessor for OnnxInferenceBlock {
    fn name(&self) -> &'static str {
        "onnx_inference"
    }

    fn prepare(&mut self, _sample_rate: f32, _block_size: usize) {
        if self.enabled {
            if let Some(ref path) = self.model_path {
                log::info!("OnnxInferenceBlock: モデル準備 (path={})", path);
            }
            // TODO: ort::Session::builder().commit_from_file(path)
        }
    }

    fn process(&mut self, _buffer: &mut [f32]) {
        if self.enabled {
            // TODO: バッファを推論スレッドに送り、出力をバッファに書き戻す。
            // 現状は identity（スタブ）なので何もしない。
        }
    }

    fn reset(&mut self) {}

    fn update_params(&mut self, cfg: &AppConfig) {
        // T-017: cfg.audio.ai_model_path が変わったらセッションを差し替える。
        let _ = cfg;
    }
}
