//! DSP チェーン。設計書 5.2.3 / 4.3。
//!
//! 処理順序（4.3）:
//!   DC カット → ノイズゲート → ピッチシフト → フォルマント補正
//!   → EQ → コンプレッサー(+メイクアップ) → リミッター
//!
//! リミッターは必ず最後（NF-005 / 5.8.3）。

use super::{
    compressor::Compressor, dc_block::DcBlock, denoise::NoiseReducer, eq::Eq, limiter::Limiter,
    noise_gate::NoiseGate, pitch_formant::PitchFormant, AudioProcessor,
};
use crate::config::AppConfig;

pub struct DspChain {
    processors: Vec<Box<dyn AudioProcessor + Send>>,
    bypass: bool,
}

impl DspChain {
    /// 設定からチェーンを構築する。
    pub fn from_config(cfg: &AppConfig, sample_rate: f32, block_size: usize) -> Self {
        let mut processors: Vec<Box<dyn AudioProcessor + Send>> = vec![
            Box::new(DcBlock::new()),
            Box::new(NoiseReducer::new(&cfg.denoise)),
            Box::new(NoiseGate::new(&cfg.noise_gate)),
            Box::new(PitchFormant::new(&cfg.voice)),
            Box::new(Eq::new(&cfg.eq)),
            Box::new(Compressor::new(&cfg.compressor)),
            Box::new(Limiter::new(&cfg.limiter)),
        ];
        for p in processors.iter_mut() {
            p.prepare(sample_rate, block_size);
        }
        Self {
            processors,
            bypass: cfg.app.bypass,
        }
    }

    /// バイパス状態を設定する。F-015。
    pub fn set_bypass(&mut self, bypass: bool) {
        self.bypass = bypass;
    }

    pub fn is_bypassed(&self) -> bool {
        self.bypass
    }

    /// バッファを順に処理する。bypass 時は素通し（5.2.3）。
    pub fn process(&mut self, buffer: &mut [f32]) {
        if self.bypass {
            return;
        }
        for processor in self.processors.iter_mut() {
            processor.process(buffer);
        }
    }

    /// 全処理の内部状態をリセットする。
    pub fn reset(&mut self) {
        for p in self.processors.iter_mut() {
            p.reset();
        }
    }

    /// 処理名の一覧（デバッグ用）。
    pub fn names(&self) -> Vec<&'static str> {
        self.processors.iter().map(|p| p.name()).collect()
    }
}
