//! KuruVoice エントリポイント。設計書 F-019 / 5.10。
//!
//! 引数なし → ダッシュボード GUI。
//! `--list-devices` / `--record-test` / `--no-gui` などは CLI 処理。

use clap::Parser;
use kuruvoice::cli::Args;
use kuruvoice::{app, gui};

fn main() {
    let args = Args::parse();

    // ログ初期化 (F-018)。--verbose で debug まで。
    let level = if args.verbose { "debug" } else { "info" };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(level)).init();

    if let Err(e) = run(&args) {
        log::error!("{e:#}");
        eprintln!("エラー: {e:#}");
        std::process::exit(1);
    }
}

fn run(args: &Args) -> anyhow::Result<()> {
    // --list-devices は最優先で処理して終了。
    if args.list_devices {
        app::list_devices();
        return Ok(());
    }

    let config = app::resolve_config(args);
    log::info!(
        "起動設定: preset={}, bypass={}, in='{}', out='{}'",
        config.app.preset,
        config.app.bypass,
        config.audio.input_device,
        config.audio.output_device
    );

    // テスト録音モード (F-017)。
    if let Some(seconds) = args.record_test {
        return app::record_test(&config, seconds);
    }

    // ヘッドレス常駐 or GUI。
    if args.no_gui {
        app::run_headless(&config)
    } else {
        gui::run(config)
    }
}
