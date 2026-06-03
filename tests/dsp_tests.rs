//! DSP 単体・結合テスト。設計書 7.1 / 7.2。

use kuruvoice::config::AppConfig;
use kuruvoice::dsp::{db_to_gain, gain_to_db, semitones_to_ratio, DspChain};

#[test]
fn db_conversion() {
    assert!((db_to_gain(0.0) - 1.0).abs() < 1e-6);
    assert!((db_to_gain(6.0) - 1.9952623).abs() < 1e-4);
    assert!((db_to_gain(-6.0) - 0.5011872).abs() < 1e-4);
    assert!(gain_to_db(1.0).abs() < 1e-4);
    // 往復
    assert!((gain_to_db(db_to_gain(-12.0)) + 12.0).abs() < 1e-3);
}

#[test]
fn pitch_ratio_formula() {
    // 5.4.3 の例
    assert!((semitones_to_ratio(-12.0) - 0.5).abs() < 1e-5);
    assert!((semitones_to_ratio(12.0) - 2.0).abs() < 1e-5);
    assert!((semitones_to_ratio(-3.0) - 0.840896).abs() < 1e-4);
}

#[test]
fn chain_runs_without_panic() {
    // 7.2 DSP Chain 実行
    let cfg = AppConfig::default();
    let mut chain = DspChain::from_config(&cfg, 48000.0, 256);
    let mut buf = vec![0.0f32; 256];
    // サイン波を入れる
    for (i, s) in buf.iter_mut().enumerate() {
        *s = (i as f32 * 0.1).sin() * 0.5;
    }
    chain.process(&mut buf);
    assert_eq!(buf.len(), 256);
    assert!(buf.iter().all(|s| s.is_finite()));
}

#[test]
fn bypass_passes_through() {
    // 7.2 バイパス切り替え: bypass=true なら入力がそのまま出る
    let mut cfg = AppConfig::default();
    cfg.app.bypass = true;
    let mut chain = DspChain::from_config(&cfg, 48000.0, 64);
    let input: Vec<f32> = (0..64).map(|i| (i as f32 * 0.3).sin() * 0.4).collect();
    let mut buf = input.clone();
    chain.process(&mut buf);
    assert_eq!(buf, input, "bypass 時は素通しのはず");
}

#[test]
fn limiter_prevents_clipping() {
    // 7.3 リミッターが効いているか（音割れ防止）
    let cfg = AppConfig::default();
    let ceiling = db_to_gain(cfg.limiter.ceiling_db);
    let mut chain = DspChain::from_config(&cfg, 48000.0, 512);
    // 過大入力
    let mut buf = vec![0.0f32; 512];
    for (i, s) in buf.iter_mut().enumerate() {
        *s = (i as f32 * 0.05).sin() * 4.0; // +12dB 相当の過大信号
    }
    chain.process(&mut buf);
    let peak = buf.iter().fold(0.0f32, |m, &s| m.max(s.abs()));
    assert!(
        peak <= ceiling + 1e-3,
        "リミッター後ピーク {peak} が天井 {ceiling} を超えた"
    );
}

#[test]
fn signal_chain_order() {
    // 4.3 / 5.8.3: リミッターは必ず最後
    let cfg = AppConfig::default();
    let chain = DspChain::from_config(&cfg, 48000.0, 128);
    let names = chain.names();
    assert_eq!(names.first(), Some(&"dc_block"));
    assert_eq!(names.last(), Some(&"limiter"));
}
