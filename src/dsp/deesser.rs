//! 動的 De-esser。
//!
//! EQ の高域シェルフは常時カットだが、この処理は歯擦音帯域が強い瞬間だけ
//! high-pass 成分を抑える。高め・女性寄りプリセットでサ行が刺さるのを防ぐ。

use super::{db_to_gain, gain_to_db, time_to_coeff, AudioProcessor};
use crate::config::DeEsserSection;
use std::f32::consts::TAU;

pub struct DeEsser {
    enabled: bool,
    split_hz: f32,
    threshold_db: f32,
    ratio: f32,
    max_reduction_db: f32,
    attack_ms: f32,
    release_ms: f32,
    attack_coeff: f32,
    release_coeff: f32,
    lowpass_coeff: f32,
    detector_low: f32,
    splitter_low: f32,
    env_db: f32,
    gain: f32,
    sample_rate: f32,
}

impl DeEsser {
    pub fn new(cfg: &DeEsserSection) -> Self {
        Self {
            enabled: cfg.enabled,
            split_hz: cfg.frequency_hz,
            threshold_db: cfg.threshold_db,
            ratio: cfg.ratio.max(1.0),
            max_reduction_db: cfg.max_reduction_db.max(0.0),
            attack_ms: cfg.attack_ms,
            release_ms: cfg.release_ms,
            attack_coeff: 0.0,
            release_coeff: 0.0,
            lowpass_coeff: 0.0,
            detector_low: 0.0,
            splitter_low: 0.0,
            env_db: -120.0,
            gain: 1.0,
            sample_rate: 48000.0,
        }
    }

    fn rebuild(&mut self) {
        let hz = self.split_hz.clamp(2500.0, self.sample_rate / 2.0 - 500.0);
        self.lowpass_coeff = (-TAU * hz / self.sample_rate).exp();
    }

    #[inline]
    fn split_high_with_coeff(coeff: f32, low_state: &mut f32, input: f32) -> f32 {
        *low_state = (1.0 - coeff) * input + coeff * *low_state;
        input - *low_state
    }
}

impl AudioProcessor for DeEsser {
    fn name(&self) -> &'static str {
        "deesser"
    }

    fn prepare(&mut self, sample_rate: f32, _block_size: usize) {
        self.sample_rate = sample_rate;
        self.attack_coeff = time_to_coeff(self.attack_ms.max(0.1), sample_rate);
        self.release_coeff = time_to_coeff(self.release_ms.max(1.0), sample_rate);
        self.rebuild();
    }

    fn process(&mut self, buffer: &mut [f32]) {
        if !self.enabled {
            return;
        }

        for sample in buffer.iter_mut() {
            let input = *sample;
            let detector_high =
                Self::split_high_with_coeff(self.lowpass_coeff, &mut self.detector_low, input);
            let level_db = gain_to_db(detector_high.abs());
            let coeff = if level_db > self.env_db {
                self.attack_coeff
            } else {
                self.release_coeff
            };
            self.env_db = level_db + coeff * (self.env_db - level_db);

            let over_db = (self.env_db - self.threshold_db).max(0.0);
            let reduction_db = (over_db * (1.0 - 1.0 / self.ratio)).min(self.max_reduction_db);
            let target_gain = db_to_gain(-reduction_db);
            self.gain = if target_gain < self.gain {
                target_gain
            } else {
                1.0 + self.release_coeff * (self.gain - 1.0)
            };

            let high =
                Self::split_high_with_coeff(self.lowpass_coeff, &mut self.splitter_low, input);
            let low = input - high;
            *sample = low + high * self.gain;
        }
    }

    fn reset(&mut self) {
        self.detector_low = 0.0;
        self.splitter_low = 0.0;
        self.env_db = -120.0;
        self.gain = 1.0;
    }
}
