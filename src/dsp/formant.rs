//! フォルマント補正。設計書 5.5 / F-008。
//!
//! 初期版は「EQ と帯域制御の組み合わせ」(5.5.3) による簡易補正。
//! ピッチを下げると声がこもりがちなので、formant_shift が負（声を下げる方向）の
//! ときに低域シェルフを軽く持ち上げ、高域シェルフを軽く下げて「太さ」を補う。
//! 正の値ではその逆。

use super::{biquad::Biquad, AudioProcessor};
use crate::config::VoiceSection;

pub struct Formant {
    enabled: bool,
    shift: f32,
    low_shelf: Biquad,
    high_shelf: Biquad,
    sample_rate: f32,
}

impl Formant {
    pub fn new(voice: &VoiceSection) -> Self {
        Self {
            enabled: voice.formant_shift.abs() > 0.01,
            shift: voice.formant_shift,
            low_shelf: Biquad::bypass(),
            high_shelf: Biquad::bypass(),
            sample_rate: 48000.0,
        }
    }

    fn rebuild(&mut self) {
        // shift 1.0 あたり ±2dB 程度の補正。
        let low_gain = -self.shift * 2.0;
        let high_gain = self.shift * 2.0;
        self.low_shelf = Biquad::low_shelf(self.sample_rate, 250.0, 0.707, low_gain);
        self.high_shelf = Biquad::high_shelf(self.sample_rate, 3500.0, 0.707, high_gain);
    }
}

impl AudioProcessor for Formant {
    fn name(&self) -> &'static str {
        "formant"
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
            let y = self.low_shelf.process(*x);
            *x = self.high_shelf.process(y);
        }
    }

    fn reset(&mut self) {
        self.low_shelf.reset();
        self.high_shelf.reset();
    }
}
