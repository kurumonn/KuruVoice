//! 音量メーター。設計書 5.x / F-016。
//!
//! ブロックのピーク・RMS を計算するヘルパーと、スレッド間で値を共有する
//! ロックフリーな `Meters`（f32 を AtomicU32 に bit 詰め）を提供する。

use std::sync::atomic::{AtomicU32, Ordering};

/// ブロックのピーク振幅（線形 0..）。
pub fn peak(buffer: &[f32]) -> f32 {
    buffer.iter().fold(0.0_f32, |m, &s| m.max(s.abs()))
}

/// ブロックの RMS（線形）。
pub fn rms(buffer: &[f32]) -> f32 {
    if buffer.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = buffer.iter().map(|s| s * s).sum();
    (sum_sq / buffer.len() as f32).sqrt()
}

/// 入力・出力レベルを GUI スレッドへ渡すための共有メーター。
#[derive(Debug, Default)]
pub struct Meters {
    input: AtomicU32,
    output: AtomicU32,
}

impl Meters {
    pub fn set_input(&self, v: f32) {
        self.input.store(v.to_bits(), Ordering::Relaxed);
    }
    pub fn set_output(&self, v: f32) {
        self.output.store(v.to_bits(), Ordering::Relaxed);
    }
    pub fn input(&self) -> f32 {
        f32::from_bits(self.input.load(Ordering::Relaxed))
    }
    pub fn output(&self) -> f32 {
        f32::from_bits(self.output.load(Ordering::Relaxed))
    }
}
