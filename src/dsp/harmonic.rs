//! ハーモニック・エンハンサー（倍音生成）。KV-DSP-1。
//!
//! ピッチを上げた女性声・中性声は倍音が疎になり「細い・芯がない・金属的」になりがち。
//! これを 2 バンドの非線形処理で補う:
//! - **芯/太さ**: 低中域(<約1.5kHz)を二乗 → **偶数次（2次）倍音**を生成し、低域シェルフ的に足す。
//! - **艶/密度**: 高域(>約2kHz)に tanh サチュレーション → **奇数次（3次）倍音**を足す。
//!
//! いずれも「生成した倍音成分のみ」を band-pass して原音に少量ミックスする（原音は保持）。
//! 時間領域・biquad のみで軽量。`enabled=false` または `amount=0` でバイパス。

use super::biquad::Biquad;
use super::AudioProcessor;
use crate::config::HarmonicSection;

pub struct HarmonicEnhancer {
    enabled: bool,
    amount: f32,
    warmth: f32,
    brightness: f32,

    // 芯（偶数次）用: 低中域を取り出す LPF と、生成倍音の DC/低域を除く HPF
    low_lp: Biquad,
    warm_hp: Biquad,
    // 艶（奇数次）用: 高域を取り出す HPF と、生成倍音を整える HPF
    high_hp: Biquad,
    bright_hp: Biquad,
    sample_rate: f32,
}

impl HarmonicEnhancer {
    pub fn new(cfg: &HarmonicSection) -> Self {
        Self {
            enabled: cfg.enabled,
            amount: cfg.amount.clamp(0.0, 1.0),
            warmth: cfg.warmth.clamp(0.0, 1.0),
            brightness: cfg.brightness.clamp(0.0, 1.0),
            low_lp: Biquad::bypass(),
            warm_hp: Biquad::bypass(),
            high_hp: Biquad::bypass(),
            bright_hp: Biquad::bypass(),
            sample_rate: 48000.0,
        }
    }

    fn rebuild(&mut self) {
        let fs = self.sample_rate;
        // 芯: 1.5kHz 以下を励起源に、生成した 2 次倍音は 200Hz 以上を残す
        self.low_lp = Biquad::low_pass(fs, 1500.0, 0.707);
        self.warm_hp = Biquad::high_pass(fs, 200.0, 0.707);
        // 艶: 2kHz 以上を励起源に、生成した倍音は 3kHz 以上を残す
        self.high_hp = Biquad::high_pass(fs, 2000.0, 0.707);
        self.bright_hp = Biquad::high_pass(fs, 3000.0, 0.707);
    }
}

impl AudioProcessor for HarmonicEnhancer {
    fn name(&self) -> &'static str {
        "harmonic"
    }

    fn prepare(&mut self, sample_rate: f32, _block_size: usize) {
        self.sample_rate = sample_rate;
        self.rebuild();
        self.reset();
    }

    fn process(&mut self, buffer: &mut [f32]) {
        if !self.enabled || self.amount <= 0.001 {
            return;
        }
        let warm_gain = self.amount * self.warmth * 1.5;
        let bright_drive = 2.0 + self.amount * 6.0;
        let bright_gain = self.amount * self.brightness * 1.0;

        for x in buffer.iter_mut() {
            let dry = *x;

            // 芯（2次倍音）: 低中域を二乗 → 偶数次(2次)倍音 + DC を生成。DC は HP で除去。
            let low = self.low_lp.process(dry);
            let even = low * low;
            let warm = self.warm_hp.process(even);

            // 艶（3次倍音）: 高域に tanh、原音帯域を引いて生成分のみ
            let high = self.high_hp.process(dry);
            let shaped = (bright_drive * high).tanh() / bright_drive;
            let bright = self.bright_hp.process(shaped - high);

            let mut y = dry + warm_gain * warm + bright_gain * bright;
            if !y.is_finite() {
                y = dry;
            }
            *x = y;
        }
    }

    fn reset(&mut self) {
        self.low_lp.reset();
        self.warm_hp.reset();
        self.high_hp.reset();
        self.bright_hp.reset();
    }

    fn update_params(&mut self, cfg: &crate::config::AppConfig) {
        self.enabled = cfg.harmonic.enabled;
        self.amount = cfg.harmonic.amount.clamp(0.0, 1.0);
        self.warmth = cfg.harmonic.warmth.clamp(0.0, 1.0);
        self.brightness = cfg.harmonic.brightness.clamp(0.0, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustfft::{num_complex::Complex, FftPlanner};
    use std::f32::consts::PI;

    fn cfg(enabled: bool, amount: f32) -> HarmonicSection {
        HarmonicSection {
            enabled,
            amount,
            warmth: 0.8,
            brightness: 0.8,
        }
    }

    /// 指定周波数のスペクトル振幅を返す。
    fn mag_at(buf: &[f32], hz: f32) -> f32 {
        let n = 16384usize;
        let start = 16000;
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(n);
        let mut spec: Vec<Complex<f32>> = (0..n)
            .map(|i| {
                let w = 0.5 * (1.0 - (2.0 * PI * i as f32 / n as f32).cos());
                Complex::new(buf[start + i] * w, 0.0)
            })
            .collect();
        fft.process(&mut spec);
        let bin = (hz / (48000.0 / n as f32)).round() as usize;
        spec[bin].norm()
    }

    #[test]
    fn disabled_is_passthrough() {
        let mut h = HarmonicEnhancer::new(&cfg(false, 0.8));
        h.prepare(48000.0, 256);
        let input: Vec<f32> = (0..512).map(|i| (i as f32 * 0.2).sin() * 0.3).collect();
        let mut buf = input.clone();
        h.process(&mut buf);
        assert_eq!(buf, input);
    }

    #[test]
    fn adds_harmonics_to_pure_tone() {
        // 純音 400Hz を入れると、倍音(800/1200Hz 付近)が増える
        let mut h = HarmonicEnhancer::new(&cfg(true, 1.0));
        h.prepare(48000.0, 256);
        let sr = 48000.0;
        let input: Vec<f32> = (0..48000)
            .map(|i| (2.0 * PI * 400.0 * i as f32 / sr).sin() * 0.4)
            .collect();
        let mut buf = input.clone();
        for b in buf.chunks_mut(256) {
            h.process(b);
        }
        assert!(buf.iter().all(|s| s.is_finite()));
        let h2_in = mag_at(&input, 800.0);
        let h2_out = mag_at(&buf, 800.0);
        assert!(
            h2_out > h2_in * 4.0 + 1e-3,
            "2次倍音(800Hz)が増えるはず: {h2_in} -> {h2_out}"
        );
    }

    #[test]
    fn output_bounded() {
        let mut h = HarmonicEnhancer::new(&cfg(true, 1.0));
        h.prepare(48000.0, 256);
        let mut buf: Vec<f32> = (0..4096)
            .map(|i| (2.0 * PI * 250.0 * i as f32 / 48000.0).sin() * 0.6)
            .collect();
        for b in buf.chunks_mut(256) {
            h.process(b);
        }
        assert!(buf.iter().all(|s| s.abs() < 3.0));
    }
}
