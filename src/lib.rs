//! KuruVoice ライブラリクレート。
//!
//! バイナリ (`main.rs`) と統合テスト (`tests/`) の双方から使えるよう、
//! コアモジュールをライブラリとして公開する。GUI は OS ウィンドウに依存するため
//! ライブラリには含めず、バイナリ側 (`src/gui`) に置く。

pub mod app;
pub mod audio;
pub mod cli;
pub mod config;
pub mod dsp;
pub mod error;
pub mod eval;
pub mod gui;
pub mod preset;
pub mod voice_character;
