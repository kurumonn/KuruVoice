//! コンプレッサー。設計書 5.7 / F-010。
//!
//! フィードフォワード型。dB ドメインのエンベロープに対して
//! threshold 超過分を ratio で圧縮し、makeup_gain を加える。

use super::{db_to_gain, gain_to_db, time_to_coeff, AudioProcessor};
use crate::config::CompressorSection;

pub struct Compressor {
    enabled: bool,
    threshold_db: f32,
    ratio: f32,
    makeup: f32, // 線形
    attack_ms: f32,
    release_ms: f32,
    attack_coeff: f32,
    release_coeff: f32,
    env_db: f32, // dB ドメインのエンベロープ
}

impl Compressor {
    pub fn new(cfg: &CompressorSection) -> Self {
        Self {
            enabled: cfg.enabled,
            threshold_db: cfg.threshold_db,
            ratio: cfg.ratio.max(1.0),
            makeup: db_to_gain(cfg.makeup_gain_db),
            attack_ms: cfg.attack_ms,
            release_ms: cfg.release_ms,
            attack_coeff: 0.0,
            release_coeff: 0.0,
            env_db: -120.0,
        }
    }
}

impl AudioProcessor for Compressor {
    fn name(&self) -> &'static str {
        "compressor"
    }

    fn prepare(&mut self, sample_rate: f32, _block_size: usize) {
        self.attack_coeff = time_to_coeff(self.attack_ms.max(0.1), sample_rate);
        self.release_coeff = time_to_coeff(self.release_ms.max(1.0), sample_rate);
    }

    fn process(&mut self, buffer: &mut [f32]) {
        if !self.enabled {
            return;
        }
        for x in buffer.iter_mut() {
            let level_db = gain_to_db(x.abs());
            // エンベロープ追従（dB）
            let coeff = if level_db > self.env_db {
                self.attack_coeff
            } else {
                self.release_coeff
            };
            self.env_db = level_db + coeff * (self.env_db - level_db);

            // 静的特性: threshold 超過分を 1/ratio に圧縮
            let over = self.env_db - self.threshold_db;
            let gain_reduction_db = if over > 0.0 {
                over * (1.0 / self.ratio - 1.0) // 負の値（減衰）
            } else {
                0.0
            };
            let gain = db_to_gain(gain_reduction_db) * self.makeup;
            *x *= gain;
        }
    }

    fn reset(&mut self) {
        self.env_db = -120.0;
    }
}
