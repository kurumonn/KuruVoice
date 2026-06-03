//! コマンドライン引数。設計書 5.10.2 / F-019。

use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "kuruvoice",
    version,
    about = "KuruVoice - 軽量リアルタイム・イケメンボイスチェンジャー",
    long_about = "DSP によりマイク音声を低遅延で「整った低音・爽やか・聞き取りやすい声」へ補正します。\n引数なしで起動するとダッシュボード GUI が開きます。"
)]
pub struct Args {
    /// 入出力デバイス一覧を表示して終了する。
    #[arg(long)]
    pub list_devices: bool,

    /// 設定ファイル(TOML)を指定する。
    #[arg(long, value_name = "FILE")]
    pub config: Option<String>,

    /// プリセットを指定する (natural_low / ikemen_soft / ikemen_deep / narrator / clear_streaming / radio_voice)。
    #[arg(long, value_name = "NAME")]
    pub preset: Option<String>,

    /// 加工なし（バイパス）で起動する。
    #[arg(long)]
    pub bypass: bool,

    /// 指定秒数だけテスト録音し、加工前後の WAV を書き出して終了する。
    #[arg(long, value_name = "SECONDS")]
    pub record_test: Option<u32>,

    /// GUI を使わずヘッドレス（CLI 常駐）で実行する。
    #[arg(long)]
    pub no_gui: bool,

    /// 詳細ログを表示する。
    #[arg(long, short)]
    pub verbose: bool,
}
