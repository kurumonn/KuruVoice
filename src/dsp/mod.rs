//! DSP（デジタル信号処理）モジュール群。設計書 5.2〜5.8。
//!
//! すべての処理は `AudioProcessor` トレイトを実装し、モノラルの `&mut [f32]`
//! ブロックをその場で加工する。処理順序は設計書 4.3 に従い `DspChain` が管理する。

pub mod auto_gain;
pub mod biquad;
pub mod chain;
pub mod compressor;
pub mod dc_block;
pub mod deesser;
pub mod denoise;
pub mod eq;
pub mod fluctuation;
pub mod harmonic;
pub mod limiter;
pub mod meter;
pub mod noise_gate;
pub mod pitch_formant;

pub use chain::DspChain;

/// 1 つの音声処理ユニット。設計書 5.2.2。
pub trait AudioProcessor {
    /// 処理名（ログ・デバッグ用）。
    fn name(&self) -> &'static str;
    /// サンプルレート・ブロックサイズが確定したときに呼ばれる初期化。
    fn prepare(&mut self, sample_rate: f32, block_size: usize);
    /// バッファをその場で加工する。
    fn process(&mut self, buffer: &mut [f32]);
    /// 内部状態（フィルタ履歴など）をクリアする。
    fn reset(&mut self);
    /// パラメータを差分更新する（内部状態はリセットしない）。T-007。
    /// デフォルト実装は何もしない。各モジュールが必要な項目だけ上書きする。
    fn update_params(&mut self, _cfg: &crate::config::AppConfig) {}
}

// ---- 共通の数学ヘルパー ----

/// dB → 線形ゲイン。
#[inline]
pub fn db_to_gain(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

/// 線形振幅 → dB。0 付近は -120dB にクランプ。
#[inline]
pub fn gain_to_db(gain: f32) -> f32 {
    if gain <= 1.0e-6 {
        -120.0
    } else {
        20.0 * gain.abs().log10()
    }
}

/// 半音 → 再生レシオ。設計書 5.4.3: `2 ^ (semitones / 12)`。
#[inline]
pub fn semitones_to_ratio(semitones: f32) -> f32 {
    2.0_f32.powf(semitones / 12.0)
}

/// attack/release の時定数(ms) から 1 次フィルタ係数を求める。
/// `coeff` は「前回値をどれだけ残すか」の係数 (0..1)。
#[inline]
pub fn time_to_coeff(time_ms: f32, sample_rate: f32) -> f32 {
    if time_ms <= 0.0 {
        0.0
    } else {
        (-1.0 / (time_ms * 0.001 * sample_rate)).exp()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_roundtrip() {
        assert!((db_to_gain(0.0) - 1.0).abs() < 1e-6);
        assert!((db_to_gain(-6.0) - 0.5011872).abs() < 1e-4);
        assert!((gain_to_db(1.0)).abs() < 1e-4);
    }

    #[test]
    fn pitch_ratio() {
        assert!((semitones_to_ratio(0.0) - 1.0).abs() < 1e-6);
        assert!((semitones_to_ratio(12.0) - 2.0).abs() < 1e-5);
        assert!((semitones_to_ratio(-12.0) - 0.5).abs() < 1e-5);
        assert!((semitones_to_ratio(-3.0) - 0.840896).abs() < 1e-4);
    }
}
