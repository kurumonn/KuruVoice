//! 音質・安定性の評価メトリクス。

use crate::dsp::{gain_to_db, meter};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioMetrics {
    pub peak: f32,
    pub rms_db: f32,
    pub clip_rate: f32,
    pub noise_floor_db: f32,
}

pub fn analyze_audio(buffer: &[f32], silence_threshold: f32) -> AudioMetrics {
    let peak = meter::peak(buffer);
    let rms_db = gain_to_db(meter::rms(buffer));
    let clip_rate = clip_rate(buffer, 0.999);
    let noise_floor_db = estimate_noise_floor_db(buffer, silence_threshold);
    AudioMetrics {
        peak,
        rms_db,
        clip_rate,
        noise_floor_db,
    }
}

pub fn clip_rate(buffer: &[f32], ceiling: f32) -> f32 {
    if buffer.is_empty() {
        return 0.0;
    }
    let clipped = buffer
        .iter()
        .filter(|sample| sample.abs() >= ceiling)
        .count();
    clipped as f32 / buffer.len() as f32
}

pub fn estimate_noise_floor_db(buffer: &[f32], silence_threshold: f32) -> f32 {
    let quiet: Vec<f32> = buffer
        .iter()
        .copied()
        .filter(|sample| sample.abs() <= silence_threshold)
        .collect();
    if quiet.is_empty() {
        return gain_to_db(meter::rms(buffer));
    }
    gain_to_db(meter::rms(&quiet))
}

pub fn percentile_latency_ms(samples_ms: &[f32], percentile: f32) -> f32 {
    if samples_ms.is_empty() {
        return 0.0;
    }
    let mut sorted: Vec<f32> = samples_ms
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .collect();
    if sorted.is_empty() {
        return 0.0;
    }
    sorted.sort_by(|a, b| a.total_cmp(b));
    let p = percentile.clamp(0.0, 100.0) / 100.0;
    let idx = (p * (sorted.len().saturating_sub(1)) as f32).round() as usize;
    sorted[idx]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_rate_counts_samples_over_ceiling() {
        let samples = [-1.0, -0.5, 0.0, 0.5, 1.0];
        assert!((clip_rate(&samples, 0.999) - 0.4).abs() < 1e-6);
    }

    #[test]
    fn percentile_latency_ignores_non_finite_values() {
        let samples = [10.0, 20.0, f32::NAN, 80.0, 40.0];
        assert_eq!(percentile_latency_ms(&samples, 95.0), 80.0);
    }
}
