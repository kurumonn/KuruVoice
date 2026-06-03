# KuruVoice 設計メモ

## 全体構成

```text
[Microphone] → [Input Device Layer (cpal)] → [Ring Buffer]
            → [DSP Processing Chain (専用スレッド)] → [Ring Buffer]
            → [Output Device Layer (cpal)] → [Speaker / 仮想デバイス / OBS]
```

cpal の入出力コールバックは **リングバッファの push/pop のみ** を行い、
重い DSP は専用スレッド (`kuruvoice-dsp`) で実行する。これにより RT コールバック内で
ロック・アロケーションを避け、安定動作（NF-004）と低遅延（NF-001）を両立する。

- 入力: 複数チャンネルをモノラルに平均してリングへ push。
- 出力: モノラルを全チャンネルへ複製。
- パラメータ更新は `mpsc` チャネル（GUI → DSP スレッド）。
- メーターは `AtomicU32`（f32 を bit 詰め）でロックフリーに共有。

## モジュール

| モジュール | 役割 |
| --- | --- |
| `audio::device` | デバイス列挙・検索（cpal） |
| `audio::engine` | ストリーム構築・処理スレッド・パラメータ更新 |
| `dsp` | `AudioProcessor` トレイトと各処理、`DspChain` |
| `dsp::biquad` | RBJ Cookbook の biquad フィルタ（EQ / フォルマント） |
| `config` | TOML スキーマと読み書き |
| `preset` | プリセット → 設定展開 |
| `cli` | コマンドライン引数 |
| `gui` | egui/eframe ダッシュボード |
| `app` | CLI/GUI 共通のオーケストレーション |

## 処理順序（4.3）

```text
DC カット → ノイズゲート → ピッチシフト → フォルマント補正
→ EQ → コンプレッサー(+メイクアップ) → リミッター
```

**リミッターは必ず最後**（NF-005 / 5.8.3）。`bypass=false` の保険として
リミッター無効時もハードクリップは残す。

## ピッチシフト方式

MVP は「2 グレイン・クロスフェード型ディレイライン」（5.4.4 の安定優先方針）。
`ratio = 2^(semitones/12)`、読み出し位相を毎サンプル `(1 - ratio)` 進め、
半ウィンドウずらした 2 つの読み出しをレイズドコサイン窓でクロスフェードする。
将来は WSOLA / Phase Vocoder / AI へ差し替え可能（トレイト境界で疎結合）。

## エラー方針（6 章）

デバイス未検出はデフォルトへフォールバック、設定不正はデフォルト設定で起動、
入力溢れ（Overrun）は古いサンプル破棄、出力枯渇（Underrun）は無音で埋める。
