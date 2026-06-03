//! ピッチシフト。設計書 5.4 / F-007。
//!
//! 初期 MVP では品質より安定動作を優先し（5.4.4）、軽量な
//! 「2 グレイン・クロスフェード型ディレイライン」方式で実装する。
//! 2 つの読み出しポインタを半ウィンドウずらして配置し、レイズドコサイン窓で
//! クロスフェードすることで、ラップ時の不連続を抑える。
//!
//! ratio = 2^(semitones/12)。読み出し位相は毎サンプル (1 - ratio) ずつ進める。

use super::{semitones_to_ratio, AudioProcessor};
use crate::config::VoiceSection;
use std::f32::consts::PI;

const WINDOW: usize = 1024; // グレイン窓長（サンプル）
const BUF_LEN: usize = 4096; // リングバッファ長（> WINDOW * 1.5）

pub struct PitchShift {
    enabled: bool,
    ratio: f32,
    mix: f32,
    buf: Vec<f32>,
    write_idx: usize,
    read_pos: f32, // 0..WINDOW の位相
}

impl PitchShift {
    pub fn new(voice: &VoiceSection) -> Self {
        Self {
            enabled: voice.pitch_semitones.abs() > 0.01,
            ratio: semitones_to_ratio(voice.pitch_semitones),
            mix: voice.mix.clamp(0.0, 1.0),
            buf: vec![0.0; BUF_LEN],
            write_idx: 0,
            read_pos: 0.0,
        }
    }

    /// バッファから線形補間で読み出す（delay は write_idx からの遅延サンプル数）。
    #[inline]
    fn read_interp(&self, delay: f32) -> f32 {
        // 読み出し位置（実数）。write_idx から delay だけ過去。
        let pos = self.write_idx as f32 - delay;
        let pos = pos.rem_euclid(BUF_LEN as f32);
        let i0 = pos.floor() as usize;
        let frac = pos - i0 as f32;
        let i1 = (i0 + 1) % BUF_LEN;
        self.buf[i0] * (1.0 - frac) + self.buf[i1] * frac
    }
}

impl AudioProcessor for PitchShift {
    fn name(&self) -> &'static str {
        "pitch_shift"
    }

    fn prepare(&mut self, _sample_rate: f32, _block_size: usize) {
        self.reset();
    }

    fn process(&mut self, buffer: &mut [f32]) {
        if !self.enabled || (self.ratio - 1.0).abs() < 1e-4 {
            return;
        }
        let w = WINDOW as f32;
        let half = w * 0.5;
        let step = 1.0 - self.ratio; // ratio<1（低く）で正、位相が前進

        for x in buffer.iter_mut() {
            // 入力を書き込む
            self.buf[self.write_idx] = *x;

            // 2 つのグレインの位相
            let frac1 = self.read_pos / w; // 0..1
            let mut frac2 = frac1 + 0.5;
            if frac2 >= 1.0 {
                frac2 -= 1.0;
            }
            // レイズドコサイン窓（和が常に 1）
            let g1 = 0.5 * (1.0 - (2.0 * PI * frac1).cos());
            let g2 = 0.5 * (1.0 - (2.0 * PI * frac2).cos());

            let d1 = self.read_pos;
            let d2 = self.read_pos + half;
            let wet = g1 * self.read_interp(d1) + g2 * self.read_interp(d2);

            *x = wet * self.mix + *x * (1.0 - self.mix);

            // 位相と書き込み位置を更新
            self.read_pos += step;
            if self.read_pos >= w {
                self.read_pos -= w;
            } else if self.read_pos < 0.0 {
                self.read_pos += w;
            }
            self.write_idx = (self.write_idx + 1) % BUF_LEN;
        }
    }

    fn reset(&mut self) {
        for s in self.buf.iter_mut() {
            *s = 0.0;
        }
        self.write_idx = 0;
        self.read_pos = 0.0;
    }
}
