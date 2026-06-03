//! ノイズゲート。設計書 5.3 / F-006。
//!
//! 入力レベルのエンベロープを追従し、しきい値未満ならゲインを下げる。
//! attack/release で急激な変化を防ぐ。

use super::{db_to_gain, time_to_coeff, AudioProcessor};
use crate::config::NoiseGateSection;

pub struct NoiseGate {
    enabled: bool,
    threshold: f32, // 線形
    attack_coeff: f32,
    release_coeff: f32,
    attack_ms: f32,
    release_ms: f32,
    env: f32,  // 入力エンベロープ（|x| 追従）
    gain: f32, // 現在の適用ゲイン
    sample_rate: f32,
}

impl NoiseGate {
    pub fn new(cfg: &NoiseGateSection) -> Self {
        Self {
            enabled: cfg.enabled,
            threshold: db_to_gain(cfg.threshold_db),
            attack_coeff: 0.0,
            release_coeff: 0.0,
            attack_ms: cfg.attack_ms,
            release_ms: cfg.release_ms,
            env: 0.0,
            gain: 1.0,
            sample_rate: 48000.0,
        }
    }
}

impl AudioProcessor for NoiseGate {
    fn name(&self) -> &'static str {
        "noise_gate"
    }

    fn prepare(&mut self, sample_rate: f32, _block_size: usize) {
        self.sample_rate = sample_rate;
        // エンベロープ追従は速め (約 2ms)、ゲインの開閉は設定の attack/release。
        self.attack_coeff = time_to_coeff(self.attack_ms.max(0.1), sample_rate);
        self.release_coeff = time_to_coeff(self.release_ms.max(1.0), sample_rate);
    }

    fn process(&mut self, buffer: &mut [f32]) {
        if !self.enabled {
            return;
        }
        // エンベロープ追従用の係数（固定 5ms）。
        let env_coeff = time_to_coeff(5.0, self.sample_rate);
        // ヒステリシス: 閉じるしきい値は開くしきい値の少し下。
        let open_th = self.threshold;
        let close_th = self.threshold * 0.5; // -6dB のヒステリシス

        for x in buffer.iter_mut() {
            let level = x.abs();
            // エンベロープ追従（1 次ローパス）
            self.env = level + env_coeff * (self.env - level);

            // 目標ゲイン: 開いていれば 1、閉じていれば 0
            let target = if self.gain > 0.5 {
                if self.env < close_th {
                    0.0
                } else {
                    1.0
                }
            } else if self.env > open_th {
                1.0
            } else {
                0.0
            };

            let coeff = if target > self.gain {
                self.attack_coeff
            } else {
                self.release_coeff
            };
            self.gain = target + coeff * (self.gain - target);
            *x *= self.gain;
        }
    }

    fn reset(&mut self) {
        self.env = 0.0;
        self.gain = 1.0;
    }
}
