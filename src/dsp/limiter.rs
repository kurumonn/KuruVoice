//! リミッター。設計書 5.8 / F-011 / NF-005。
//!
//! チェーンの最終段に必ず配置し、音割れを防ぐ（安全装置）。
//! 即時アタックで天井を超えないようゲインを下げ、release_ms で戻す。

use super::{db_to_gain, time_to_coeff, AudioProcessor};
use crate::config::LimiterSection;

pub struct Limiter {
    enabled: bool,
    ceiling: f32, // 線形
    release_coeff: f32,
    release_ms: f32,
    gain: f32,
}

impl Limiter {
    pub fn new(cfg: &LimiterSection) -> Self {
        Self {
            enabled: cfg.enabled,
            ceiling: db_to_gain(cfg.ceiling_db),
            release_coeff: 0.0,
            release_ms: cfg.release_ms,
            gain: 1.0,
        }
    }
}

impl AudioProcessor for Limiter {
    fn name(&self) -> &'static str {
        "limiter"
    }

    fn prepare(&mut self, sample_rate: f32, _block_size: usize) {
        self.release_coeff = time_to_coeff(self.release_ms.max(1.0), sample_rate);
    }

    fn process(&mut self, buffer: &mut [f32]) {
        // リミッターは安全装置のため、enabled=false でもハードクリップだけは行う。
        for x in buffer.iter_mut() {
            if self.enabled {
                let peak = x.abs() * self.gain;
                if peak > self.ceiling {
                    // 必要な瞬時ゲインまで即座に下げる（アタック 0）
                    let target = self.ceiling / x.abs().max(1e-9);
                    self.gain = target.min(self.gain);
                } else {
                    // release で 1.0 方向へ戻す
                    self.gain = 1.0 + self.release_coeff * (self.gain - 1.0);
                }
                *x *= self.gain;
            }
            // 最終ハードクリップ（理論上の保険）
            let c = self.ceiling;
            *x = x.clamp(-c, c);
        }
    }

    fn reset(&mut self) {
        self.gain = 1.0;
    }
}
