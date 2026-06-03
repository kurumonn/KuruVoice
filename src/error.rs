//! KuruVoice エラー型。
//!
//! 設計書 第6章「エラー設計」に対応する。想定エラーを列挙し、
//! 上位ではできるだけフォールバック（デフォルト設定・デフォルトデバイス）で
//! 復帰し、アプリ全体が panic で落ちないようにする (NF-004)。

use thiserror::Error;

/// KuruVoice 全体で使う想定エラー。
#[derive(Error, Debug)]
pub enum KuruError {
    #[error("入力デバイスが見つかりません: {0}")]
    InputDeviceNotFound(String),

    #[error("出力デバイスが見つかりません: {0}")]
    OutputDeviceNotFound(String),

    #[error("サンプルレート {0} Hz はサポートされていません")]
    UnsupportedSampleRate(u32),

    #[error("設定ファイルの読み込みに失敗しました: {0}")]
    ConfigLoadError(String),

    #[error("設定ファイルの保存に失敗しました: {0}")]
    ConfigSaveError(String),

    #[error("DSP 処理エラー: {0}")]
    DspError(String),

    #[error("オーディオデバイスエラー: {0}")]
    DeviceError(String),

    #[error("オーディオストリームの構築に失敗しました: {0}")]
    StreamBuildError(String),
}

/// クレート共通の Result 型。
pub type Result<T> = std::result::Result<T, KuruError>;
