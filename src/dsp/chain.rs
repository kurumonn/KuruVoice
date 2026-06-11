//! DSP チェーン。設計書 5.2.3 / 4.3。
//!
//! 処理順序（4.3 拡張）:
//!   DC カット → ノイズ低減 → ノイズゲート → オートゲイン(AGC)
//!   → ピッチ/フォルマント → ゆらぎ → EQ → ハーモニック → De-esser
//!   → コンプレッサー(+メイクアップ) → リミッター
//!
//! 「綺麗に入れる(補正) → 綺麗に変える(声質) → 綺麗に出す(整音)」。
//! リミッターは必ず最後（NF-005 / 5.8.3）。

use super::{
    auto_gain::AutoGain, compressor::Compressor, dc_block::DcBlock, deesser::DeEsser,
    denoise::NoiseReducer, eq::Eq, fluctuation::Fluctuation, harmonic::HarmonicEnhancer,
    limiter::Limiter, noise_gate::NoiseGate, pitch_formant::PitchFormant, AudioProcessor,
};
use crate::config::AppConfig;

/// バイパスフェード 1 サンプルあたりの追従係数（約 20ms でフェード完了）。
const BYPASS_ALPHA: f32 = 0.005;

pub struct DspChain {
    processors: Vec<Box<dyn AudioProcessor + Send>>,
    /// 0.0 = フル処理、1.0 = フルバイパス（目標値）。
    bypass_target: f32,
    /// 現在のフェード位置（クロスフェード中に 0↔1 を補間）。
    bypass_fade: f32,
    /// ドライ信号を一時保持するバッファ（クロスフェード用。事前確保でアロケーション回避）。
    dry_buf: Vec<f32>,
}

impl DspChain {
    /// 設定からチェーンを構築する。
    pub fn from_config(cfg: &AppConfig, sample_rate: f32, block_size: usize) -> Self {
        let mut processors: Vec<Box<dyn AudioProcessor + Send>> = vec![
            Box::new(DcBlock::new()),
            Box::new(NoiseReducer::new(&cfg.denoise)),
            Box::new(NoiseGate::new(&cfg.noise_gate)),
            Box::new(AutoGain::new(&cfg.auto_gain)),
            Box::new(PitchFormant::new(&cfg.voice)),
            Box::new(Fluctuation::new(&cfg.fluctuation)),
            Box::new(Eq::new(&cfg.eq)),
            Box::new(HarmonicEnhancer::new(&cfg.harmonic)),
            Box::new(DeEsser::new(&cfg.deesser)),
            Box::new(Compressor::new(&cfg.compressor)),
            Box::new(Limiter::new(&cfg.limiter)),
        ];
        for p in processors.iter_mut() {
            p.prepare(sample_rate, block_size);
        }
        let bypass_val = if cfg.app.bypass { 1.0 } else { 0.0 };
        Self {
            processors,
            bypass_target: bypass_val,
            bypass_fade: bypass_val,
            dry_buf: vec![0.0; block_size],
        }
    }

    /// パラメータを差分更新する（チェーンを再構築しない）。T-007。
    /// GUI のスライダー操作時に呼び出し、プチノイズを防ぐ。
    pub fn update_params(&mut self, cfg: &AppConfig) {
        for p in self.processors.iter_mut() {
            p.update_params(cfg);
        }
        self.bypass_target = if cfg.app.bypass { 1.0 } else { 0.0 };
    }

    /// バイパス状態を設定する。F-015。クロスフェード付き。
    pub fn set_bypass(&mut self, bypass: bool) {
        self.bypass_target = if bypass { 1.0 } else { 0.0 };
    }

    pub fn is_bypassed(&self) -> bool {
        self.bypass_target > 0.5
    }

    /// バッファを順に処理する。bypass 時はクロスフェードで素通しへ移行（5.2.3 / T-009）。
    pub fn process(&mut self, buffer: &mut [f32]) {
        let at_bypass = self.bypass_fade > 0.999 && self.bypass_target > 0.999;
        let at_wet = self.bypass_fade < 0.001 && self.bypass_target < 0.001;

        if at_bypass {
            return;
        }

        if at_wet {
            for processor in self.processors.iter_mut() {
                processor.process(buffer);
            }
            return;
        }

        // クロスフェード中: ドライコピーを保持してから処理し、ブレンド
        let len = buffer.len().min(self.dry_buf.len());
        self.dry_buf[..len].copy_from_slice(&buffer[..len]);

        for processor in self.processors.iter_mut() {
            processor.process(buffer);
        }

        for (sample, dry_sample) in buffer.iter_mut().zip(self.dry_buf.iter()).take(len) {
            self.bypass_fade += (self.bypass_target - self.bypass_fade) * BYPASS_ALPHA;
            let fade = self.bypass_fade.clamp(0.0, 1.0);
            *sample = *sample * (1.0 - fade) + *dry_sample * fade;
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
