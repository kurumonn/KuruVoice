//! EQ。設計書 5.6 / F-009。
//!
//! 帯域設計 (5.6.2):
//! - high_pass_hz 以下      : カット（ハイパス）
//! - 150Hz 付近            : 低音感（ローシェルフ low_boost_db）
//! - 400Hz 付近            : こもり（ピーキング mud_cut_db）
//! - 4kHz 付近             : 明瞭感（ピーキング presence_boost_db）
//! - 7.5kHz 付近           : 歯擦音（ハイシェルフ de_esser_db）

use super::{biquad::Biquad, AudioProcessor};
use crate::config::EqSection;

pub struct Eq {
    enabled: bool,
    cfg: EqSection,
    high_pass: Biquad,
    low_boost: Biquad,
    mud_cut: Biquad,
    presence: Biquad,
    de_esser: Biquad,
    sample_rate: f32,
}

impl Eq {
    pub fn new(cfg: &EqSection) -> Self {
        Self {
            enabled: cfg.enabled,
            cfg: cfg.clone(),
            high_pass: Biquad::bypass(),
            low_boost: Biquad::bypass(),
            mud_cut: Biquad::bypass(),
            presence: Biquad::bypass(),
            de_esser: Biquad::bypass(),
            sample_rate: 48000.0,
        }
    }

    fn rebuild(&mut self) {
        let fs = self.sample_rate;
        let hp = self.cfg.high_pass_hz.clamp(20.0, fs / 2.0 - 100.0);
        self.high_pass = Biquad::high_pass(fs, hp, 0.707);
        self.low_boost = Biquad::low_shelf(fs, 150.0, 0.707, self.cfg.low_boost_db);
        self.mud_cut = Biquad::peaking(fs, 400.0, 1.0, self.cfg.mud_cut_db);
        self.presence = Biquad::peaking(fs, 4000.0, 1.0, self.cfg.presence_boost_db);
        // de_esser はこの静的 EQ ではハイシェルフによる歯擦音抑制で近似する。
        self.de_esser = Biquad::high_shelf(fs, 7500.0, 0.707, self.cfg.de_esser_db);
    }
}

impl AudioProcessor for Eq {
    fn name(&self) -> &'static str {
        "eq"
    }

    fn prepare(&mut self, sample_rate: f32, _block_size: usize) {
        self.sample_rate = sample_rate;
        self.rebuild();
    }

    fn process(&mut self, buffer: &mut [f32]) {
        if !self.enabled {
            return;
        }
        for x in buffer.iter_mut() {
            let mut y = self.high_pass.process(*x);
            y = self.low_boost.process(y);
            y = self.mud_cut.process(y);
            y = self.presence.process(y);
            y = self.de_esser.process(y);
            *x = y;
        }
    }

    fn reset(&mut self) {
        self.high_pass.reset();
        self.low_boost.reset();
        self.mud_cut.reset();
        self.presence.reset();
        self.de_esser.reset();
    }
}
