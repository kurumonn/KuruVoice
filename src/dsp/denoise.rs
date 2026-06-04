//! ノイズキャンセル（STFT スペクトル減算）。F-006 の拡張。
//!
//! ノイズゲート（無音時に音を切る）とは別に、**発話中も含めて定常的な背景ノイズ**
//! （PC ファン・エアコン・ホワイトノイズ等）を周波数ごとに低減する。
//!
//! 方式: 各周波数ビンの振幅から、最小値追従で推定したノイズ床を差し引く
//! （オーバーサブトラクション + 下限ゲイン）。位相は保持し、音楽性ノイズを抑えるため
//! ゲインを時間方向に平滑化する。`enabled=false` または `amount=0` でバイパス。

use super::AudioProcessor;
use crate::config::DenoiseSection;
use rustfft::{num_complex::Complex, Fft, FftPlanner};
use std::f32::consts::PI;
use std::sync::Arc;

const FFT_SIZE: usize = 1024;
const OVERSAMP: usize = 4;
const HOP: usize = FFT_SIZE / OVERSAMP;
const HALF: usize = FFT_SIZE / 2;
const NORM: f32 = 1.0 / (1.5 * FFT_SIZE as f32);
const NOISE_RISE: f32 = 1.0008; // ノイズ床がゆっくり上昇する係数（過小推定の固着防止）

fn czero() -> Complex<f32> {
    Complex::new(0.0, 0.0)
}

pub struct NoiseReducer {
    enabled: bool,
    amount: f32,
    gmin: f32, // 最大減衰時の下限ゲイン
    over: f32, // オーバーサブトラクション係数

    fwd: Arc<dyn Fft<f32>>,
    inv: Arc<dyn Fft<f32>>,
    scratch: Vec<Complex<f32>>,

    window: Vec<f32>,
    in_fifo: Vec<f32>,
    out_fifo: Vec<f32>,
    out_accum: Vec<f32>,
    rover: usize,

    spec: Vec<Complex<f32>>,
    noise: Vec<f32>,
    gain_prev: Vec<f32>,
    initialized: bool,
}

impl NoiseReducer {
    pub fn new(cfg: &DenoiseSection) -> Self {
        let mut planner = FftPlanner::<f32>::new();
        let fwd = planner.plan_fft_forward(FFT_SIZE);
        let inv = planner.plan_fft_inverse(FFT_SIZE);
        let scratch_len = fwd
            .get_inplace_scratch_len()
            .max(inv.get_inplace_scratch_len());
        let window: Vec<f32> = (0..FFT_SIZE)
            .map(|k| 0.5 * (1.0 - (2.0 * PI * k as f32 / FFT_SIZE as f32).cos()))
            .collect();
        let amount = cfg.amount.clamp(0.0, 1.0);
        Self {
            enabled: cfg.enabled,
            amount,
            gmin: 10.0_f32.powf(-24.0 * amount / 20.0), // amount=1 → -24dB まで
            over: 1.0 + amount * 1.5,
            fwd,
            inv,
            scratch: vec![czero(); scratch_len],
            window,
            in_fifo: vec![0.0; FFT_SIZE],
            out_fifo: vec![0.0; FFT_SIZE],
            out_accum: vec![0.0; 2 * FFT_SIZE],
            rover: FFT_SIZE - HOP,
            spec: vec![czero(); FFT_SIZE],
            noise: vec![1.0e6; HALF + 1],
            gain_prev: vec![1.0; HALF + 1],
            initialized: false,
        }
    }

    fn bypassed(&self) -> bool {
        !self.enabled || self.amount <= 0.001
    }

    fn process_frame(&mut self) {
        for k in 0..FFT_SIZE {
            self.spec[k] = Complex::new(self.in_fifo[k] * self.window[k], 0.0);
        }
        self.fwd
            .process_with_scratch(&mut self.spec, &mut self.scratch);

        for k in 0..=HALF {
            let mag = self.spec[k].norm();
            // ノイズ床の最小値追従（下げは即時、上げは緩やか）
            if mag < self.noise[k] {
                self.noise[k] = mag;
            } else {
                self.noise[k] *= NOISE_RISE;
            }
            // スペクトル減算 → ゲイン
            let sub = mag - self.over * self.noise[k];
            let mut g = if mag > 1e-9 { sub / mag } else { self.gmin };
            g = g.clamp(self.gmin, 1.0);
            // 時間方向に平滑化（musical noise 抑制）
            self.gain_prev[k] = 0.5 * self.gain_prev[k] + 0.5 * g;
            self.spec[k] *= self.gain_prev[k];
        }
        self.initialized = true;

        // 共役対称
        for k in 1..HALF {
            self.spec[FFT_SIZE - k] = self.spec[k].conj();
        }
        self.spec[0].im = 0.0;
        self.spec[HALF].im = 0.0;

        self.inv
            .process_with_scratch(&mut self.spec, &mut self.scratch);

        for k in 0..FFT_SIZE {
            self.out_accum[k] += self.window[k] * self.spec[k].re;
        }
        for k in 0..HOP {
            self.out_fifo[k] = self.out_accum[k] * NORM;
        }
        self.out_accum.copy_within(HOP..2 * FFT_SIZE, 0);
        for v in self.out_accum[(2 * FFT_SIZE - HOP)..].iter_mut() {
            *v = 0.0;
        }
        self.in_fifo.copy_within(HOP..FFT_SIZE, 0);
    }
}

impl AudioProcessor for NoiseReducer {
    fn name(&self) -> &'static str {
        "denoise"
    }

    fn prepare(&mut self, _sample_rate: f32, _block_size: usize) {
        self.reset();
    }

    fn process(&mut self, buffer: &mut [f32]) {
        if self.bypassed() {
            return;
        }
        let latency = FFT_SIZE - HOP;
        for x in buffer.iter_mut() {
            self.in_fifo[self.rover] = *x;
            let wet = self.out_fifo[self.rover - latency];
            self.rover += 1;
            if self.rover >= FFT_SIZE {
                self.rover = latency;
                self.process_frame();
            }
            *x = if wet.is_finite() { wet } else { 0.0 };
        }
    }

    fn reset(&mut self) {
        for v in self.in_fifo.iter_mut() {
            *v = 0.0;
        }
        for v in self.out_fifo.iter_mut() {
            *v = 0.0;
        }
        for v in self.out_accum.iter_mut() {
            *v = 0.0;
        }
        for v in self.noise.iter_mut() {
            *v = 1.0e6;
        }
        for v in self.gain_prev.iter_mut() {
            *v = 1.0;
        }
        self.initialized = false;
        self.rover = FFT_SIZE - HOP;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(enabled: bool, amount: f32) -> DenoiseSection {
        DenoiseSection { enabled, amount }
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt()
    }

    #[test]
    fn disabled_is_passthrough() {
        let mut nr = NoiseReducer::new(&cfg(false, 0.8));
        nr.prepare(48000.0, 256);
        let input: Vec<f32> = (0..512).map(|i| (i as f32 * 0.2).sin() * 0.3).collect();
        let mut buf = input.clone();
        for b in buf.chunks_mut(256) {
            nr.process(b);
        }
        assert_eq!(buf, input);
    }

    #[test]
    fn attenuates_white_noise() {
        let mut nr = NoiseReducer::new(&cfg(true, 1.0));
        nr.prepare(48000.0, 256);
        // 疑似ホワイトノイズ（線形合同法）
        let mut seed: u32 = 12345;
        let mut buf: Vec<f32> = (0..48000)
            .map(|_| {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                (seed >> 9) as f32 / (1u32 << 23) as f32 - 1.0
            })
            .map(|v| v * 0.3)
            .collect();
        let in_rms = rms(&buf[24000..40000]);
        for b in buf.chunks_mut(256) {
            nr.process(b);
        }
        let out_rms = rms(&buf[24000..40000]);
        assert!(
            out_rms < in_rms * 0.8,
            "ノイズが低減されるはず: {in_rms} -> {out_rms}"
        );
    }

    #[test]
    fn keeps_strong_tone() {
        let mut nr = NoiseReducer::new(&cfg(true, 1.0));
        nr.prepare(48000.0, 256);
        let sr = 48000.0;
        let mut buf: Vec<f32> = (0..48000)
            .map(|i| (2.0 * PI * 300.0 * i as f32 / sr).sin() * 0.5)
            .collect();
        let in_rms = rms(&buf[24000..40000]);
        for b in buf.chunks_mut(256) {
            nr.process(b);
        }
        let out_rms = rms(&buf[24000..40000]);
        assert!(
            out_rms > in_rms * 0.5,
            "強いトーンは概ね保持される: {in_rms} -> {out_rms}"
        );
        assert!(buf.iter().all(|s| s.is_finite()));
    }
}
