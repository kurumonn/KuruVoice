//! ノイズキャンセル（STFT スペクトル減算 + VAD 連動ノイズ推定）。F-006 の拡張 / KV-IN-5。
//!
//! ノイズゲート（無音時に音を切る）とは別に、**発話中も含めて定常的な背景ノイズ**
//! （PC ファン・エアコン・ホワイトノイズ等）を周波数ごとに低減する。
//!
//! ノイズ床の推定は **VAD（音声区間検出）連動**:
//! - フレーム全体のエネルギーが「最近の平均より十分小さい」＝静かなフレームのときだけ
//!   各周波数ビンのノイズ床を学習する（EMA）。
//! - 声が鳴っている間（大きいフレーム）は学習を凍結する。
//!
//! これにより、**伸ばした母音・ハミングなどの持続音をノイズと誤判定して抑制してしまう問題**
//! （旧・最小値追従が開始フレームをノイズ床に固定する不具合）を解消する。学習前（静かな
//! フレーム未観測）は素通し。`enabled=false` または `amount=0` でバイパス。

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
const QUIET_FACTOR: f32 = 0.6; // 平均エネルギーの何倍以下を「静か(=ノイズ)」とみなすか
const LEARN: f32 = 0.1; // 静かなフレームでのノイズ床学習速度
const AVG_COEFF: f32 = 0.99; // 長期平均エネルギーの追従（遅め＝発話の起伏より長い時定数）

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
    mag: Vec<f32>,
    noise: Vec<f32>,
    gain_prev: Vec<f32>,
    avg_energy: f32,
    frames: u32,
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
            mag: vec![0.0; HALF + 1],
            noise: vec![0.0; HALF + 1], // 学習前は 0 ＝ 素通し
            gain_prev: vec![1.0; HALF + 1],
            avg_energy: 0.0,
            frames: 0,
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

        // フレームエネルギーと振幅
        let mut energy = 0.0f32;
        for k in 0..=HALF {
            let m = self.spec[k].norm();
            self.mag[k] = m;
            energy += m * m;
        }
        energy /= (HALF + 1) as f32;

        // 長期平均エネルギーの更新と VAD 判定
        if self.frames == 0 {
            self.avg_energy = energy;
        } else {
            self.avg_energy = self.avg_energy * AVG_COEFF + energy * (1.0 - AVG_COEFF);
        }
        self.frames += 1;
        let is_quiet = energy <= self.avg_energy * QUIET_FACTOR;

        for k in 0..=HALF {
            // 静かなフレームでだけノイズ床を学習（声が鳴っている間は凍結）
            if is_quiet {
                self.noise[k] = self.noise[k] * (1.0 - LEARN) + self.mag[k] * LEARN;
            }
            let mag = self.mag[k];
            let sub = mag - self.over * self.noise[k];
            let mut g = if mag > 1e-9 { sub / mag } else { self.gmin };
            g = g.clamp(self.gmin, 1.0);
            self.gain_prev[k] = 0.5 * self.gain_prev[k] + 0.5 * g;
            self.spec[k] *= self.gain_prev[k];
        }

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
            *v = 0.0;
        }
        for v in self.gain_prev.iter_mut() {
            *v = 1.0;
        }
        self.avg_energy = 0.0;
        self.frames = 0;
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

    fn noise_sample(seed: &mut u32) -> f32 {
        *seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        (*seed >> 9) as f32 / (1u32 << 23) as f32 - 1.0
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

    /// KV-IN-5: 持続音（伸ばした母音相当の定常トーン）は抑制せず保持する。
    #[test]
    fn keeps_sustained_tone() {
        let mut nr = NoiseReducer::new(&cfg(true, 1.0));
        nr.prepare(48000.0, 256);
        let sr = 48000.0;
        let mut buf: Vec<f32> = (0..48000)
            .map(|i| (2.0 * PI * 300.0 * i as f32 / sr).sin() * 0.4)
            .collect();
        let in_rms = rms(&buf[24000..40000]);
        for b in buf.chunks_mut(256) {
            nr.process(b);
        }
        let out_rms = rms(&buf[24000..40000]);
        assert!(buf.iter().all(|s| s.is_finite()));
        assert!(
            out_rms > in_rms * 0.8,
            "持続音は保持されるべき: {in_rms} -> {out_rms}"
        );
    }

    /// 背景ノイズ（途中に大きい音＝発話相当がある）の静かな区間が低減されること。
    #[test]
    fn attenuates_background_noise_in_quiet_region() {
        let mut nr = NoiseReducer::new(&cfg(true, 1.0));
        nr.prepare(48000.0, 256);
        let sr = 48000.0;
        let mut seed = 12345u32;
        // 3秒: 全体に弱いノイズ + [1.0s,2.0s] に大きいトーン(発話相当)
        let mut buf: Vec<f32> = (0..144000)
            .map(|i| {
                let t = i as f32 / sr;
                let mut s = noise_sample(&mut seed) * 0.08;
                if (1.0..2.0).contains(&t) {
                    s += (2.0 * PI * 220.0 * t).sin() * 0.5;
                }
                s
            })
            .collect();
        // 計測: トーン後の静かなノイズ区間 [2.4s, 2.9s]
        let lo = (sr * 2.4) as usize;
        let hi = (sr * 2.9) as usize;
        let in_rms = rms(&buf[lo..hi]);
        for b in buf.chunks_mut(256) {
            nr.process(b);
        }
        let out_rms = rms(&buf[lo..hi]);
        assert!(buf.iter().all(|s| s.is_finite()));
        assert!(
            out_rms < in_rms * 0.85,
            "静音区間の背景ノイズは低減されるべき: {in_rms} -> {out_rms}"
        );
    }
}
