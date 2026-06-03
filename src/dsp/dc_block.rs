//! DC カット。設計書 4.3 のチェーン先頭。1 次ハイパスで直流成分を除去する。

use super::AudioProcessor;

pub struct DcBlock {
    r: f32,
    x_prev: f32,
    y_prev: f32,
}

impl DcBlock {
    pub fn new() -> Self {
        Self {
            r: 0.995,
            x_prev: 0.0,
            y_prev: 0.0,
        }
    }
}

impl Default for DcBlock {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioProcessor for DcBlock {
    fn name(&self) -> &'static str {
        "dc_block"
    }

    fn prepare(&mut self, sample_rate: f32, _block_size: usize) {
        // カットオフをおよそ 20Hz に保つよう係数を調整。
        let fc = 20.0_f32;
        self.r = (-2.0 * std::f32::consts::PI * fc / sample_rate).exp();
    }

    fn process(&mut self, buffer: &mut [f32]) {
        for x in buffer.iter_mut() {
            let xn = *x;
            let yn = xn - self.x_prev + self.r * self.y_prev;
            self.x_prev = xn;
            self.y_prev = yn;
            *x = yn;
        }
    }

    fn reset(&mut self) {
        self.x_prev = 0.0;
        self.y_prev = 0.0;
    }
}
