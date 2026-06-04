//! 高品質ピッチシフト + 独立フォルマントシフト（位相ボコーダ）。設計書 5.4 / 5.5 の高品質版。
//!
//! 旧来の粒状ディレイライン方式（artifacts が出やすい）を置き換える。
//! STFT（短時間フーリエ変換）ベースの位相ボコーダで、
//! - **ピッチ**: 周波数ビンを `2^(semitones/12)` 倍にマッピングし、位相を伝播して再合成。
//! - **フォルマント**: ケプストラムでスペクトル包絡を推定し、励起（ピッチ成分）と分離。
//!   包絡を `2^(formant_shift/6)` 倍に伸縮して再付与することで、ピッチと独立に
//!   「声の太さ/細さ」を変えられる（大きく上げても自然＝チップマンク化しない）。
//!
//! 処理はホストのブロックサイズに依存せず、内部で FFT_SIZE / HOP のフレーム処理を行う
//! ストリーミング OLA。`pitch=formant=neutral` のときはバイパス（CPU/遅延ゼロ）。
//! アクティブ時は約 FFT_SIZE-HOP サンプル（48kHz で約 16ms）の遅延が加わる。

use super::{semitones_to_ratio, AudioProcessor};
use crate::config::VoiceSection;
use rustfft::{num_complex::Complex, Fft, FftPlanner};
use std::f32::consts::PI;
use std::sync::Arc;

const FFT_SIZE: usize = 1024;
const OVERSAMP: usize = 4;
const HOP: usize = FFT_SIZE / OVERSAMP; // 256
const HALF: usize = FFT_SIZE / 2; // 512
const LIFTER: usize = 40; // ケプストラム・リフター幅（包絡の滑らかさ）
const NORM: f32 = 1.0 / (1.5 * FFT_SIZE as f32); // Hann 75% OLA + 非正規化 iFFT の補正

fn czero() -> Complex<f32> {
    Complex::new(0.0, 0.0)
}

pub struct PitchFormant {
    active: bool,
    pitch_ratio: f32,
    formant_ratio: f32,
    mix: f32,
    sample_rate: f32,
    freq_per_bin: f32,
    expct: f32,

    fwd: Arc<dyn Fft<f32>>,
    inv: Arc<dyn Fft<f32>>,
    scratch: Vec<Complex<f32>>,

    window: Vec<f32>,
    in_fifo: Vec<f32>,
    out_fifo: Vec<f32>,
    out_accum: Vec<f32>,
    last_phase: Vec<f32>,
    sum_phase: Vec<f32>,
    rover: usize,

    spec: Vec<Complex<f32>>,
    cep: Vec<Complex<f32>>,
    ana_magn: Vec<f32>,
    ana_freq: Vec<f32>,
    syn_magn: Vec<f32>,
    syn_freq: Vec<f32>,
    env: Vec<f32>,
    white: Vec<f32>,
    syn_white: Vec<f32>,

    dry_delay: Vec<f32>,
    dry_pos: usize,
}

impl PitchFormant {
    pub fn new(voice: &VoiceSection) -> Self {
        let mut planner = FftPlanner::<f32>::new();
        let fwd = planner.plan_fft_forward(FFT_SIZE);
        let inv = planner.plan_fft_inverse(FFT_SIZE);
        let scratch_len = fwd
            .get_inplace_scratch_len()
            .max(inv.get_inplace_scratch_len());
        let window: Vec<f32> = (0..FFT_SIZE)
            .map(|k| 0.5 * (1.0 - (2.0 * PI * k as f32 / FFT_SIZE as f32).cos()))
            .collect();
        let latency = FFT_SIZE - HOP;

        Self {
            active: voice.pitch_semitones.abs() > 0.01 || voice.formant_shift.abs() > 0.01,
            pitch_ratio: semitones_to_ratio(voice.pitch_semitones),
            formant_ratio: 2.0_f32.powf(voice.formant_shift / 6.0),
            mix: voice.mix.clamp(0.0, 1.0),
            sample_rate: 48000.0,
            freq_per_bin: 48000.0 / FFT_SIZE as f32,
            expct: 2.0 * PI * HOP as f32 / FFT_SIZE as f32,
            fwd,
            inv,
            scratch: vec![czero(); scratch_len],
            window,
            in_fifo: vec![0.0; FFT_SIZE],
            out_fifo: vec![0.0; FFT_SIZE],
            out_accum: vec![0.0; 2 * FFT_SIZE],
            last_phase: vec![0.0; HALF + 1],
            sum_phase: vec![0.0; HALF + 1],
            rover: latency,
            spec: vec![czero(); FFT_SIZE],
            cep: vec![czero(); FFT_SIZE],
            ana_magn: vec![0.0; HALF + 1],
            ana_freq: vec![0.0; HALF + 1],
            syn_magn: vec![0.0; HALF + 1],
            syn_freq: vec![0.0; HALF + 1],
            env: vec![0.0; HALF + 1],
            white: vec![0.0; HALF + 1],
            syn_white: vec![0.0; HALF + 1],
            dry_delay: vec![0.0; latency],
            dry_pos: 0,
        }
    }

    fn bypassed(&self) -> bool {
        !self.active
            || ((self.pitch_ratio - 1.0).abs() < 1e-4 && (self.formant_ratio - 1.0).abs() < 1e-4)
    }

    /// 1 フレーム（FFT_SIZE）分の解析・加工・合成と OLA。
    fn process_frame(&mut self) {
        // 窓掛け → FFT
        for k in 0..FFT_SIZE {
            self.spec[k] = Complex::new(self.in_fifo[k] * self.window[k], 0.0);
        }
        self.fwd
            .process_with_scratch(&mut self.spec, &mut self.scratch);

        // 解析: 振幅と「真の周波数」
        for k in 0..=HALF {
            let re = self.spec[k].re;
            let im = self.spec[k].im;
            let magn = (re * re + im * im).sqrt();
            let phase = im.atan2(re);
            let mut tmp = phase - self.last_phase[k];
            self.last_phase[k] = phase;
            tmp -= k as f32 * self.expct;
            // ±π に折り返し
            let mut qpd = (tmp / PI) as i32;
            if qpd >= 0 {
                qpd += qpd & 1;
            } else {
                qpd -= qpd & 1;
            }
            tmp -= PI * qpd as f32;
            tmp = OVERSAMP as f32 * tmp / (2.0 * PI);
            self.ana_magn[k] = magn;
            self.ana_freq[k] = (k as f32 + tmp) * self.freq_per_bin;
        }

        // スペクトル包絡（ケプストラム・リフター）
        self.compute_envelope();

        // 励起の白色化 → ピッチシフト → 包絡を伸縮して再付与
        for k in 0..=HALF {
            self.white[k] = self.ana_magn[k] / (self.env[k] + 1e-9);
            self.syn_white[k] = 0.0;
            self.syn_freq[k] = 0.0;
        }
        for k in 0..=HALF {
            let index = (k as f32 * self.pitch_ratio).round() as usize;
            if index <= HALF {
                self.syn_white[index] += self.white[k];
                self.syn_freq[index] = self.ana_freq[k] * self.pitch_ratio;
            }
        }
        for k in 0..=HALF {
            let src = ((k as f32 / self.formant_ratio).round() as isize).clamp(0, HALF as isize);
            self.syn_magn[k] = self.syn_white[k] * self.env[src as usize];
        }

        // 合成: 位相を蓄積して複素スペクトルを再構成
        for k in 0..=HALF {
            let mut tmp = self.syn_freq[k];
            tmp -= k as f32 * self.freq_per_bin;
            tmp /= self.freq_per_bin;
            tmp = 2.0 * PI * tmp / OVERSAMP as f32;
            tmp += k as f32 * self.expct;
            self.sum_phase[k] += tmp;
            let phase = self.sum_phase[k];
            self.spec[k] = Complex::new(
                self.syn_magn[k] * phase.cos(),
                self.syn_magn[k] * phase.sin(),
            );
        }
        // 共役対称で全スペクトルを埋める
        for k in 1..HALF {
            self.spec[FFT_SIZE - k] = self.spec[k].conj();
        }
        self.spec[0].im = 0.0;
        self.spec[HALF].im = 0.0;

        self.inv
            .process_with_scratch(&mut self.spec, &mut self.scratch);

        // 窓掛けオーバーラップ加算
        for k in 0..FFT_SIZE {
            self.out_accum[k] += self.window[k] * self.spec[k].re;
        }
        for k in 0..HOP {
            self.out_fifo[k] = self.out_accum[k] * NORM;
        }
        // アキュムレータと入力 FIFO を HOP 分ずらす
        self.out_accum.copy_within(HOP..2 * FFT_SIZE, 0);
        for v in self.out_accum[(2 * FFT_SIZE - HOP)..].iter_mut() {
            *v = 0.0;
        }
        self.in_fifo.copy_within(HOP..FFT_SIZE, 0);
    }

    /// ケプストラム・リフターでスペクトル包絡 `env[k]` を推定する。
    fn compute_envelope(&mut self) {
        for k in 0..=HALF {
            let lm = self.ana_magn[k].max(1e-6).ln();
            self.cep[k] = Complex::new(lm, 0.0);
        }
        for k in 1..HALF {
            self.cep[FFT_SIZE - k] = Complex::new(self.cep[k].re, 0.0);
        }
        // iFFT → ケプストラム（非正規化なので /N）
        self.inv
            .process_with_scratch(&mut self.cep, &mut self.scratch);
        let invn = 1.0 / FFT_SIZE as f32;
        for c in self.cep.iter_mut() {
            *c *= invn;
        }
        // 低ケフレンシのみ残す（包絡＝滑らかな対数振幅）
        for (q, c) in self.cep.iter_mut().enumerate() {
            if q > LIFTER && q < FFT_SIZE - LIFTER {
                *c = czero();
            }
        }
        // FFT → 平滑化された対数振幅 → exp で包絡
        self.fwd
            .process_with_scratch(&mut self.cep, &mut self.scratch);
        for k in 0..=HALF {
            self.env[k] = self.cep[k].re.exp();
        }
    }
}

impl AudioProcessor for PitchFormant {
    fn name(&self) -> &'static str {
        "pitch_formant"
    }

    fn prepare(&mut self, sample_rate: f32, _block_size: usize) {
        self.sample_rate = sample_rate;
        self.freq_per_bin = sample_rate / FFT_SIZE as f32;
        self.reset();
    }

    fn process(&mut self, buffer: &mut [f32]) {
        if self.bypassed() {
            return;
        }
        let latency = FFT_SIZE - HOP;
        for x in buffer.iter_mut() {
            let dry = *x;
            self.in_fifo[self.rover] = dry;
            let wet = self.out_fifo[self.rover - latency];
            self.rover += 1;
            if self.rover >= FFT_SIZE {
                self.rover = latency;
                self.process_frame();
            }
            // ドライをウェットと同じ遅延だけ遅らせて mix
            let delayed_dry = self.dry_delay[self.dry_pos];
            self.dry_delay[self.dry_pos] = dry;
            self.dry_pos = (self.dry_pos + 1) % self.dry_delay.len();

            let mut y = wet * self.mix + delayed_dry * (1.0 - self.mix);
            if !y.is_finite() {
                y = 0.0;
            }
            *x = y;
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
        for v in self.last_phase.iter_mut() {
            *v = 0.0;
        }
        for v in self.sum_phase.iter_mut() {
            *v = 0.0;
        }
        for v in self.dry_delay.iter_mut() {
            *v = 0.0;
        }
        self.dry_pos = 0;
        self.rover = FFT_SIZE - HOP;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn voice(pitch: f32, formant: f32) -> VoiceSection {
        VoiceSection {
            pitch_semitones: pitch,
            formant_shift: formant,
            mix: 1.0,
        }
    }

    /// 正弦波を流し込み、定常部の最強スペクトル成分（支配周波数）を FFT で推定。
    fn run_and_detect(pf: &mut PitchFormant, in_hz: f32) -> f32 {
        let sr = 48000.0;
        pf.prepare(sr, 256);
        let total = 48000; // 1 秒
        let mut out = vec![0.0f32; total];
        for (i, s) in out.iter_mut().enumerate() {
            *s = (2.0 * PI * in_hz * i as f32 / sr).sin() * 0.5;
        }
        for b in out.chunks_mut(256) {
            pf.process(b);
        }
        // 定常部から N サンプルを Hann 窓 + FFT し、最大振幅ビンの周波数を返す。
        let n = 16384usize;
        let start = 24000;
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(n);
        let mut buf: Vec<Complex<f32>> = (0..n)
            .map(|i| {
                let w = 0.5 * (1.0 - (2.0 * PI * i as f32 / n as f32).cos());
                Complex::new(out[start + i] * w, 0.0)
            })
            .collect();
        fft.process(&mut buf);
        let mut best = 0.0f32;
        let mut best_bin = 0usize;
        for (k, c) in buf.iter().enumerate().take(n / 2).skip(1) {
            let m = c.norm();
            if m > best {
                best = m;
                best_bin = k;
            }
        }
        best_bin as f32 * sr / n as f32
    }

    #[test]
    fn octave_up_doubles_pitch() {
        let mut pf = PitchFormant::new(&voice(12.0, 0.0));
        let f = run_and_detect(&mut pf, 200.0);
        assert!((f - 400.0).abs() < 30.0, "+12半音で約400Hzになるはず: {f}");
    }

    #[test]
    fn octave_down_halves_pitch() {
        let mut pf = PitchFormant::new(&voice(-12.0, 0.0));
        let f = run_and_detect(&mut pf, 200.0);
        assert!((f - 100.0).abs() < 20.0, "-12半音で約100Hzになるはず: {f}");
    }

    #[test]
    fn formant_only_keeps_pitch() {
        // ピッチはそのまま(±0)・フォルマントだけ上げる → 基本周波数は保たれる
        let mut pf = PitchFormant::new(&voice(0.0, 3.0));
        assert!(!pf.bypassed(), "フォルマントのみでも処理は有効");
        let f = run_and_detect(&mut pf, 200.0);
        assert!(
            (f - 200.0).abs() < 20.0,
            "フォルマント変更でピッチは不変: {f}"
        );
    }

    #[test]
    fn neutral_is_bypassed() {
        let pf = PitchFormant::new(&voice(0.0, 0.0));
        assert!(pf.bypassed());
    }

    #[test]
    fn output_is_finite_and_bounded() {
        let mut pf = PitchFormant::new(&voice(7.0, -2.0));
        pf.prepare(48000.0, 256);
        let mut buf: Vec<f32> = (0..48000)
            .map(|i| (2.0 * PI * 150.0 * i as f32 / 48000.0).sin() * 0.5)
            .collect();
        for b in buf.chunks_mut(256) {
            pf.process(b);
        }
        assert!(buf.iter().all(|s| s.is_finite()));
        assert!(buf.iter().all(|s| s.abs() < 4.0), "極端に増幅されないこと");
    }
}
