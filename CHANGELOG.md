# Changelog

All notable changes to KuruVoice are documented here.
This project adheres to [Semantic Versioning](https://semver.org/).

## [0.1.1] - 2026-06-11

### Added
- Windows 向け zip 配布に README / LICENSE / config.example.toml / docs を同梱。
- Inno Setup による `KuruVoiceSetup-vX.Y.Z-windows-x64.exe` 配布を追加。
- Release Assets に SHA256 ファイルを追加。
- GitHub Actions で Windows / Linux / macOS 配布物を自動生成。
- 任意の Windows コード署名フローを追加（GitHub Secrets 設定時のみ署名）。
- 配布手順とコード署名手順のドキュメントを追加。

### Fixed
- `cargo clippy --all-targets -- -D warnings` が通るよう既存警告を解消。

## [0.1.0] - 2026-06-04

### Added
- 初回リリース（MVP）。
- オーディオ入出力（cpal）、リングバッファ + 専用 DSP スレッド構成。
- DSP チェーン: DC カット / ノイズゲート / ピッチシフト / フォルマント補正 / EQ / コンプレッサー / リミッター。
- 6 プリセット: Natural Low / Ikemen Soft / Ikemen Deep / Narrator / Clear Streaming / Radio Voice。
- TOML 設定の読み書き（部分指定可）。
- ダッシュボード GUI（egui/eframe）: デバイス選択・プリセット・スライダー・入出力メーター・バイパス・保存/読込。
- CLI: `--list-devices` / `--config` / `--preset` / `--bypass` / `--record-test` / `--no-gui` / `--verbose`。
- 単体・結合テスト（dB 変換 / ピッチ比 / チェーン / バイパス / リミッター / 設定 / プリセット）。
- ドキュメント: README, docs/design・usage・preset・safety, examples/obs_setup。
- GitHub Actions（CI / Release）。
