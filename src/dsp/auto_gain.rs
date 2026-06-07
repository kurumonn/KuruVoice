//! オートゲイン（AGC）。KV-IN-2 / FR-002。
//!
//! 入力 RMS を追従し、目標レベルへ**緩やかに**利得を合わせる。
//! - 大きい入力 → 速めに利得を下げる（クリップ回避）
//! - 小さい入力 → ゆっくり持ち上げる（ポンピング回避）。ただし `max_gain_db` で頭打ち
//! - ゲート以下（無音）→ 利得を**保持**してノイズを増幅しない
//!
//! 最終段のリミッターと併用して過大音量を防ぐ。`enabled=false` でバイパス。

use super::{db_to_gain, time_to_coeff, AudioProcessor};
use crate::config::AutoGainSection;

pub struct AutoGain {
    enabled: bool,
    target: f32,   // 目標 RMS（線形）
    max_gain: f32, // 線形
    min_gain: f32, // 線形（下げ過ぎ防止: -12dB）
    gate: f32,     // 線形ゲート
    env: f32,      // 入力パワーのエンベロープ (x^2 の EMA)
    gain: f32,
    env_coeff: f32,
    up_coeff: f32,
    down_coeff: f32,
}

impl AutoGain {
    pub fn new(cfg: &AutoGainSection) -> Self {
        Self {
            enabled: cfg.enabled,
            target: db_to_gain(cfg.target_db),
            max_gain: db_to_gain(cfg.max_gain_db.max(0.0)),
            min_gain: db_to_gain(-12.0),
            gate: db_to_gain(cfg.gate_db),
            env: 0.0,
            gain: 1.0,
            env_coeff: 0.0,
            up_coeff: 0.0,
            down_coeff: 0.0,
        }
    }
}

impl AudioProcessor for AutoGain {
    fn name(&self) -> &'static str {
        "auto_gain"
    }

    fn prepare(&mut self, sample_rate: f32, _block_size: usize) {
        self.env_coeff = time_to_coeff(30.0, sample_rate); // RMS 追従 ~30ms
        self.up_coeff = time_to_coeff(400.0, sample_rate); // ブーストはゆっくり
        self.down_coeff = time_to_coeff(100.0, sample_rate); // 抑制は速め
        self.env = 0.0;
        self.gain = 1.0;
    }

    fn process(&mut self, buffer: &mut [f32]) {
        if !self.enabled {
            return;
        }
        for x in buffer.iter_mut() {
            let p = *x * *x;
            self.env = p + self.env_coeff * (self.env - p);
            let level = self.env.sqrt();

            let desired = if level > self.gate {
                (self.target / level).clamp(self.min_gain, self.max_gain)
            } else {
                // 無音域では現状の利得を保持（ノイズを持ち上げない）
                self.gain
            };
            let coeff = if desired < self.gain {
                self.down_coeff
            } else {
                self.up_coeff
            };
            self.gain = desired + coeff * (self.gain - desired);
            *x *= self.gain;
        }
    }

    fn reset(&mut self) {
        self.env = 0.0;
        self.gain = 1.0;
    }

    fn update_params(&mut self, cfg: &crate::config::AppConfig) {
        self.enabled = cfg.auto_gain.enabled;
        self.target = db_to_gain(cfg.auto_gain.target_db);
        self.max_gain = db_to_gain(cfg.auto_gain.max_gain_db.max(0.0));
        self.gate = db_to_gain(cfg.auto_gain.gate_db);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::gain_to_db;
    use std::f32::consts::TAU;

    fn cfg(enabled: bool) -> AutoGainSection {
        AutoGainSection {
            enabled,
            target_db: -20.0,
            max_gain_db: 18.0,
            gate_db: -50.0,
        }
    }

    fn sine(amp: f32, n: usize) -> Vec<f32> {
        (0..n)
            .map(|i| (TAU * 300.0 * i as f32 / 48000.0).sin() * amp)
            .collect()
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt()
    }

    #[test]
    fn disabled_is_passthrough() {
        let mut ag = AutoGain::new(&cfg(false));
        ag.prepare(48000.0, 256);
        let input = sine(0.05, 512);
        let mut buf = input.clone();
        for b in buf.chunks_mut(256) {
            ag.process(b);
        }
        assert_eq!(buf, input);
    }

    #[test]
    fn boosts_quiet_input() {
        let mut ag = AutoGain::new(&cfg(true));
        ag.prepare(48000.0, 256);
        // -40dB 程度の小さい入力
        let mut buf = sine(0.01, 48000);
        let in_rms = rms(&buf[0..4096]);
        for b in buf.chunks_mut(256) {
            ag.process(b);
        }
        let out_rms = rms(&buf[40000..44000]);
        assert!(buf.iter().all(|s| s.is_finite()));
        assert!(
            out_rms > in_rms * 2.0,
            "小さい声は持ち上がる: {in_rms} -> {out_rms}"
        );
        // 目標(-20dB)を大きく超えない（max_gain で頭打ち）
        assert!(gain_to_db(out_rms) <= -18.0, "目標を超えて増幅しない");
    }

    #[test]
    fn reduces_loud_input() {
        let mut ag = AutoGain::new(&cfg(true));
        ag.prepare(48000.0, 256);
        let mut buf = sine(0.5, 48000); // 大きい入力
        let in_rms = rms(&buf[0..4096]);
        for b in buf.chunks_mut(256) {
            ag.process(b);
        }
        let out_rms = rms(&buf[40000..44000]);
        assert!(
            out_rms < in_rms,
            "大きい声は抑えられる: {in_rms} -> {out_rms}"
        );
    }

    #[test]
    fn does_not_amplify_silence() {
        let mut ag = AutoGain::new(&cfg(true));
        ag.prepare(48000.0, 256);
        // ゲート(-50dB)未満のごく小さいノイズ的入力
        let mut buf = sine(0.001, 48000);
        for b in buf.chunks_mut(256) {
            ag.process(b);
        }
        let out_rms = rms(&buf[40000..44000]);
        assert!(out_rms < 0.01, "無音域は増幅しない: {out_rms}");
    }
}
