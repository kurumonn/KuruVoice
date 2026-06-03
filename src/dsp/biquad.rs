//! 2 次 IIR（biquad）フィルタ。RBJ Audio EQ Cookbook の係数を使用。
//! EQ・フォルマント補正で共用する。

use std::f32::consts::PI;

/// Transposed Direct Form II の biquad。
#[derive(Debug, Clone, Copy, Default)]
pub struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Biquad {
    /// 係数を正規化して設定する（a0 で割る）。
    fn set(&mut self, b0: f32, b1: f32, b2: f32, a0: f32, a1: f32, a2: f32) {
        self.b0 = b0 / a0;
        self.b1 = b1 / a0;
        self.b2 = b2 / a0;
        self.a1 = a1 / a0;
        self.a2 = a2 / a0;
    }

    /// 何もしない（ゲイン 1.0）フィルタ。
    pub fn bypass() -> Self {
        let mut q = Self::default();
        q.set(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);
        q
    }

    /// 1 サンプル処理。
    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y
    }

    /// フィルタ履歴をクリア。
    pub fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }

    pub fn high_pass(fs: f32, f0: f32, q: f32) -> Self {
        let mut bq = Self::default();
        let w0 = 2.0 * PI * (f0 / fs);
        let (sin, cos) = w0.sin_cos();
        let alpha = sin / (2.0 * q);
        let b0 = (1.0 + cos) / 2.0;
        let b1 = -(1.0 + cos);
        let b2 = (1.0 + cos) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos;
        let a2 = 1.0 - alpha;
        bq.set(b0, b1, b2, a0, a1, a2);
        bq
    }

    #[allow(dead_code)]
    pub fn low_pass(fs: f32, f0: f32, q: f32) -> Self {
        let mut bq = Self::default();
        let w0 = 2.0 * PI * (f0 / fs);
        let (sin, cos) = w0.sin_cos();
        let alpha = sin / (2.0 * q);
        let b1 = 1.0 - cos;
        let b0 = b1 / 2.0;
        let b2 = b0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos;
        let a2 = 1.0 - alpha;
        bq.set(b0, b1, b2, a0, a1, a2);
        bq
    }

    /// ピーキング EQ（特定帯域をブースト/カット）。
    pub fn peaking(fs: f32, f0: f32, q: f32, gain_db: f32) -> Self {
        let mut bq = Self::default();
        let a = 10.0_f32.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * (f0 / fs);
        let (sin, cos) = w0.sin_cos();
        let alpha = sin / (2.0 * q);
        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * cos;
        let b2 = 1.0 - alpha * a;
        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * cos;
        let a2 = 1.0 - alpha / a;
        bq.set(b0, b1, b2, a0, a1, a2);
        bq
    }

    /// ローシェルフ。
    pub fn low_shelf(fs: f32, f0: f32, q: f32, gain_db: f32) -> Self {
        let mut bq = Self::default();
        let a = 10.0_f32.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * (f0 / fs);
        let (sin, cos) = w0.sin_cos();
        let alpha = sin / (2.0 * q);
        let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
        let b0 = a * ((a + 1.0) - (a - 1.0) * cos + two_sqrt_a_alpha);
        let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos);
        let b2 = a * ((a + 1.0) - (a - 1.0) * cos - two_sqrt_a_alpha);
        let a0 = (a + 1.0) + (a - 1.0) * cos + two_sqrt_a_alpha;
        let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos);
        let a2 = (a + 1.0) + (a - 1.0) * cos - two_sqrt_a_alpha;
        bq.set(b0, b1, b2, a0, a1, a2);
        bq
    }

    /// ハイシェルフ。
    pub fn high_shelf(fs: f32, f0: f32, q: f32, gain_db: f32) -> Self {
        let mut bq = Self::default();
        let a = 10.0_f32.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * (f0 / fs);
        let (sin, cos) = w0.sin_cos();
        let alpha = sin / (2.0 * q);
        let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
        let b0 = a * ((a + 1.0) + (a - 1.0) * cos + two_sqrt_a_alpha);
        let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos);
        let b2 = a * ((a + 1.0) + (a - 1.0) * cos - two_sqrt_a_alpha);
        let a0 = (a + 1.0) - (a - 1.0) * cos + two_sqrt_a_alpha;
        let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos);
        let a2 = (a + 1.0) - (a - 1.0) * cos - two_sqrt_a_alpha;
        bq.set(b0, b1, b2, a0, a1, a2);
        bq
    }
}
