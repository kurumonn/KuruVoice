//! 「声の印象」マクロ調整。
//!
//! DSP の専門用語（ピッチ/フォルマント/EQ）を知らなくても、
//! 「明瞭さ・かわいさ・かっこよさ・怖さ」という直感的な用語（0..1）で声を変えられる。
//! GUI ではこれをレーダーチャート（グラフ）でドラッグ調整する。
//!
//! 各軸は複数の DSP パラメータの組み合わせにマップされる（`apply_to_config`）。

use crate::config::AppConfig;

/// 声の印象（各軸 0.0..1.0）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VoiceCharacter {
    /// 明瞭さ（高いほど明るく聞き取りやすい）
    pub clarity: f32,
    /// かわいさ（高いほど高く・軽い声）
    pub cuteness: f32,
    /// かっこよさ（落ち着いた・芯のある低音）
    pub coolness: f32,
    /// 怖さ（暗く・低く・重い声）
    pub fear: f32,
}

impl Default for VoiceCharacter {
    fn default() -> Self {
        // 初期表示用のバランス値（変更するまで設定には適用されない）。
        Self {
            clarity: 0.4,
            cuteness: 0.2,
            coolness: 0.4,
            fear: 0.1,
        }
    }
}

impl VoiceCharacter {
    /// レーダーチャートの軸ラベル（描画順）。
    pub const AXES: [&'static str; 4] = ["明瞭さ", "かわいさ", "かっこよさ", "怖さ"];

    /// 軸の値を描画順で返す。
    pub fn values(&self) -> [f32; 4] {
        [self.clarity, self.cuteness, self.coolness, self.fear]
    }

    /// 軸 i の値を設定する（0..1 にクランプ）。
    pub fn set(&mut self, i: usize, v: f32) {
        let v = v.clamp(0.0, 1.0);
        match i {
            0 => self.clarity = v,
            1 => self.cuteness = v,
            2 => self.coolness = v,
            3 => self.fear = v,
            _ => {}
        }
    }

    /// 印象パラメータを実際の DSP 設定（声づくり + EQ）に反映する。
    pub fn apply_to_config(&self, cfg: &mut AppConfig) {
        let c = self.clarity;
        let q = self.cuteness;
        let k = self.coolness;
        let f = self.fear;

        // ピッチ: かわいい=高く / かっこいい=やや低く / 怖い=大きく低く
        // 高品質ピッチ/フォルマントになったので可動域を広めに取る。
        cfg.voice.pitch_semitones = (q * 9.0 - k * 4.0 - f * 12.0).clamp(-24.0, 24.0);
        // フォルマント: かわいい=上げ(細く) / 怖い=下げ(太く) / かっこいい=やや下げ
        cfg.voice.formant_shift = (q * 4.0 - f * 4.0 - k * 1.0).clamp(-12.0, 12.0);

        cfg.eq.enabled = true;
        // 明瞭感: 明瞭さ・かっこよさで増、怖さで減
        cfg.eq.presence_boost_db = (2.0 + c * 5.0 + k * 1.5 - f * 2.5).clamp(-6.0, 8.0);
        // 低音: かっこよさ・怖さで増、明瞭さで減
        cfg.eq.low_boost_db = (k * 1.5 + f * 2.5 - c * 1.0).clamp(-6.0, 6.0);
        // ローカット: 明瞭さ・かわいさで高め（軽く）
        cfg.eq.high_pass_hz = (80.0 + c * 30.0 + q * 20.0).clamp(20.0, 300.0);
        // こもりカット: 明瞭さ・かわいさで強める
        cfg.eq.mud_cut_db = (-1.0 - c * 2.0 - q * 1.0).clamp(-12.0, 6.0);
        // 歯擦音: 怖さで抑える / 明瞭さでやや戻す
        cfg.eq.de_esser_db = (-2.0 - f * 1.5 + c * 0.5).clamp(-12.0, 0.0);

        // 任意のマクロ操作なのでプリセット表示は「カスタム」に。
        cfg.app.preset = "custom".to_string();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ch(clarity: f32, cuteness: f32, coolness: f32, fear: f32) -> VoiceCharacter {
        VoiceCharacter {
            clarity,
            cuteness,
            coolness,
            fear,
        }
    }

    #[test]
    fn fear_lowers_pitch() {
        let mut cfg = AppConfig::default();
        ch(0.0, 0.0, 0.0, 1.0).apply_to_config(&mut cfg);
        assert!(cfg.voice.pitch_semitones < -3.0, "怖さは声を大きく下げる");
        assert!(
            cfg.voice.formant_shift < 0.0,
            "怖さは太く(フォルマント下げ)"
        );
    }

    #[test]
    fn cuteness_raises_pitch() {
        let mut cfg = AppConfig::default();
        ch(0.0, 1.0, 0.0, 0.0).apply_to_config(&mut cfg);
        assert!(cfg.voice.pitch_semitones > 0.0, "かわいさは声を上げる");
        assert!(
            cfg.voice.formant_shift > 0.0,
            "かわいさは細く(フォルマント上げ)"
        );
    }

    #[test]
    fn clarity_boosts_presence() {
        let mut cfg = AppConfig::default();
        ch(1.0, 0.0, 0.0, 0.0).apply_to_config(&mut cfg);
        assert!(cfg.eq.presence_boost_db > 5.0, "明瞭さは明瞭感を上げる");
    }

    #[test]
    fn coolness_is_low_and_present() {
        let mut cfg = AppConfig::default();
        ch(0.0, 0.0, 1.0, 0.0).apply_to_config(&mut cfg);
        assert!(cfg.voice.pitch_semitones < 0.0, "かっこよさはやや低い");
        assert!(cfg.eq.low_boost_db > 0.0, "かっこよさは低音を足す");
    }

    #[test]
    fn values_and_set_roundtrip() {
        let mut c = VoiceCharacter::default();
        c.set(3, 0.9);
        assert_eq!(c.values()[3], 0.9);
        c.set(3, 2.0); // クランプ
        assert_eq!(c.values()[3], 1.0);
    }
}
