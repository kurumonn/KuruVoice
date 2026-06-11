//! 1/f ゆらぎモード。KV-DSP-5。
//!
//! 自然な声は、ピッチや音量がランダムだが 1/f（ピンク）スペクトルでゆっくり揺らいでいる。
//! これを模して、ピンクノイズで駆動する **微小ピッチ揺れ（マイクロ・ビブラート）** と
//! **音量揺れ（トレモロ）** を加え、平坦・機械的な合成感を消して「人間らしい・心地よい」
//! 声にする。
//!
//! - ピッチ揺れ: 変調ディレイライン（フラクショナル遅延）で実現。
//! - 音量揺れ: ゆっくりしたゲイン変調。
//! - 駆動信号: Paul Kellet のピンクノイズ近似を rate_hz 付近へ 1 次 LPF して使用。
//!
//! `enabled=false` または `amount=0` でバイパス。

use super::{time_to_coeff, AudioProcessor};
use crate::config::FluctuationSection;
use std::f32::consts::TAU;

/// Paul Kellet 近似のピンク(1/f)ノイズ生成器。
struct Pink {
    rng: u32,
    b: [f32; 7],
}

impl Pink {
    fn new(seed: u32) -> Self {
        Self {
            rng: seed | 1,
            b: [0.0; 7],
        }
    }

    #[inline]
    fn white(&mut self) -> f32 {
        // xorshift32
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        (x as f32 / u32::MAX as f32) * 2.0 - 1.0
    }

    #[inline]
    #[allow(clippy::excessive_precision, clippy::unreadable_literal)]
    fn next(&mut self) -> f32 {
        let w = self.white();
        self.b[0] = 0.99886 * self.b[0] + w * 0.0555179;
        self.b[1] = 0.99332 * self.b[1] + w * 0.0750759;
        self.b[2] = 0.96900 * self.b[2] + w * 0.1538520;
        self.b[3] = 0.86650 * self.b[3] + w * 0.3104856;
        self.b[4] = 0.55000 * self.b[4] + w * 0.5329522;
        self.b[5] = -0.7616 * self.b[5] - w * 0.0168980;
        let pink = self.b[0]
            + self.b[1]
            + self.b[2]
            + self.b[3]
            + self.b[4]
            + self.b[5]
            + self.b[6]
            + w * 0.5362;
        self.b[6] = w * 0.115926;
        pink * 0.11
    }
}

const CTRL_NORM: f32 = 3.0; // LPF 後のピンク制御を概ね ±1 に正規化する係数

pub struct Fluctuation {
    enabled: bool,
    amount: f32,
    amp_depth: f32,
    pitch_cents: f32,
    rate_hz: f32,
    sample_rate: f32,

    p_pitch: Pink,
    p_amp: Pink,
    ctrl_pitch: f32,
    ctrl_amp: f32,
    lp_coeff: f32,

    buf: Vec<f32>,
    widx: usize,
    base_delay: f32,
    depth_samples: f32,
}

impl Fluctuation {
    pub fn new(cfg: &FluctuationSection) -> Self {
        Self {
            enabled: cfg.enabled,
            amount: cfg.amount.clamp(0.0, 1.0),
            amp_depth: cfg.amp_depth.clamp(0.0, 1.0),
            pitch_cents: cfg.pitch_cents.max(0.0),
            rate_hz: cfg.rate_hz.clamp(0.1, 20.0),
            sample_rate: 48000.0,
            p_pitch: Pink::new(0x1234_5678),
            p_amp: Pink::new(0x9E37_79B9),
            ctrl_pitch: 0.0,
            ctrl_amp: 0.0,
            lp_coeff: 0.0,
            buf: vec![0.0; 1024],
            widx: 0,
            base_delay: 64.0,
            depth_samples: 0.0,
        }
    }

    fn bypassed(&self) -> bool {
        !self.enabled || self.amount <= 0.001
    }
}

impl AudioProcessor for Fluctuation {
    fn name(&self) -> &'static str {
        "fluctuation"
    }

    fn prepare(&mut self, sample_rate: f32, _block_size: usize) {
        self.sample_rate = sample_rate;
        // rate_hz を 1 次 LPF のカットオフとして使い、揺らぎの速さを決める。
        self.lp_coeff = time_to_coeff(1000.0 / self.rate_hz, sample_rate);
        // セント目標から変調ディレイ深さ(サンプル)を求める。
        // ピッチ偏差(cents) ≈ (1200/ln2) * Δdelay/sample, Δdelay/sample ≈ depth * (2π rate/fs)
        let depth = self.pitch_cents * sample_rate / (1731.0 * TAU * self.rate_hz) * self.amount;
        self.depth_samples = depth.clamp(0.0, 200.0);
        self.base_delay = self.depth_samples + 16.0;
        let needed = (self.base_delay + self.depth_samples + 8.0) as usize;
        let len = needed.next_power_of_two().max(256);
        self.buf = vec![0.0; len];
        self.widx = 0;
        self.ctrl_pitch = 0.0;
        self.ctrl_amp = 0.0;
    }

    fn process(&mut self, buffer: &mut [f32]) {
        if self.bypassed() {
            return;
        }
        let len = self.buf.len();
        for x in buffer.iter_mut() {
            self.buf[self.widx] = *x;

            // 1/f 制御信号（LPF で rate 付近へ）
            let pp = self.p_pitch.next();
            self.ctrl_pitch = pp + self.lp_coeff * (self.ctrl_pitch - pp);
            let pa = self.p_amp.next();
            self.ctrl_amp = pa + self.lp_coeff * (self.ctrl_amp - pa);
            let cp = (self.ctrl_pitch * CTRL_NORM).clamp(-1.0, 1.0);
            let ca = (self.ctrl_amp * CTRL_NORM).clamp(-1.0, 1.0);

            // 変調ディレイで微小ピッチ揺れ
            let delay = self.base_delay + self.depth_samples * cp;
            let rp = (self.widx as f32 - delay).rem_euclid(len as f32);
            let i0 = rp.floor() as usize;
            let frac = rp - i0 as f32;
            let i1 = (i0 + 1) % len;
            let vib = self.buf[i0] * (1.0 - frac) + self.buf[i1] * frac;

            // 音量揺れ
            let g = 1.0 + self.amp_depth * self.amount * ca;
            let mut y = vib * g;
            if !y.is_finite() {
                y = *x;
            }
            *x = y;

            self.widx = (self.widx + 1) % len;
        }
    }

    fn reset(&mut self) {
        for v in self.buf.iter_mut() {
            *v = 0.0;
        }
        self.widx = 0;
        self.ctrl_pitch = 0.0;
        self.ctrl_amp = 0.0;
    }

    fn update_params(&mut self, cfg: &crate::config::AppConfig) {
        self.enabled = cfg.fluctuation.enabled;
        self.amount = cfg.fluctuation.amount.clamp(0.0, 1.0);
        self.amp_depth = cfg.fluctuation.amp_depth.clamp(0.0, 1.0);
        self.pitch_cents = cfg.fluctuation.pitch_cents.max(0.0);
        self.rate_hz = cfg.fluctuation.rate_hz.clamp(0.1, 20.0);
        if self.sample_rate > 0.0 {
            self.lp_coeff = time_to_coeff(1000.0 / self.rate_hz, self.sample_rate);
            let depth =
                self.pitch_cents * self.sample_rate / (1731.0 * TAU * self.rate_hz) * self.amount;
            self.depth_samples = depth.clamp(0.0, 200.0);
            self.base_delay = self.depth_samples + 16.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustfft::{num_complex::Complex, FftPlanner};

    fn cfg(enabled: bool, pitch_cents: f32, amp_depth: f32) -> FluctuationSection {
        FluctuationSection {
            enabled,
            amount: 1.0,
            pitch_cents,
            amp_depth,
            rate_hz: 6.0,
        }
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt()
    }

    #[test]
    fn pink_is_low_frequency_weighted() {
        // 1/f なら低域エネルギー > 高域エネルギー
        let mut p = Pink::new(42);
        let n = 32768usize;
        let sr = 48000.0;
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(n);
        let mut spec: Vec<Complex<f32>> = (0..n).map(|_| Complex::new(p.next(), 0.0)).collect();
        fft.process(&mut spec);
        let bin = sr / n as f32;
        let band = |lo: f32, hi: f32| -> f32 {
            let a = (lo / bin) as usize;
            let b = (hi / bin) as usize;
            spec[a..b].iter().map(|c| c.norm_sqr()).sum::<f32>()
        };
        let low = band(20.0, 200.0);
        let high = band(2000.0, 10000.0);
        // 1/f は概ねオクターブ等エネルギー → 低域(約3.3oct)/高域(約2.3oct) で低域優位。
        // 白色なら low/high << 1 になるので 1.2 倍でも十分に判別できる。
        assert!(
            low > high * 1.2,
            "ピンクは低域優位のはず: low={low} high={high}"
        );
    }

    #[test]
    fn disabled_is_passthrough() {
        let mut f = Fluctuation::new(&cfg(false, 12.0, 0.1));
        f.prepare(48000.0, 256);
        let input: Vec<f32> = (0..512).map(|i| (i as f32 * 0.2).sin() * 0.3).collect();
        let mut buf = input.clone();
        for b in buf.chunks_mut(256) {
            f.process(b);
        }
        assert_eq!(buf, input);
    }

    #[test]
    fn amplitude_fluctuates_over_time() {
        // 定常正弦に音量揺れを掛けると、時間で RMS が変動する
        let mut f = Fluctuation::new(&cfg(true, 0.0, 0.5));
        f.prepare(48000.0, 256);
        let mut buf: Vec<f32> = (0..96000)
            .map(|i| (TAU * 300.0 * i as f32 / 48000.0).sin() * 0.3)
            .collect();
        for b in buf.chunks_mut(256) {
            f.process(b);
        }
        assert!(buf.iter().all(|s| s.is_finite()));
        assert!(buf.iter().all(|s| s.abs() < 2.0));
        // 窓ごとの RMS の変動を確認
        let win = 4800;
        let levels: Vec<f32> = buf[24000..].chunks(win).map(rms).collect();
        let max = levels.iter().cloned().fold(0.0f32, f32::max);
        let min = levels.iter().cloned().fold(f32::MAX, f32::min);
        let mean = levels.iter().sum::<f32>() / levels.len() as f32;
        assert!(
            (max - min) / mean > 0.03,
            "音量が揺らいでいるはず: max={max} min={min}"
        );
    }
}
