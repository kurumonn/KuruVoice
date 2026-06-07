# KuruVoice 改修ロードマップ

> レビュー受領: 2026-06-07  
> ベース commit: 現行 main（v0.1.0）

---

## 優先度凡例

| 記号 | 意味 |
|------|------|
| 🔴 P0 | 即修正（リリースブロッカー / ユーザー体験を壊す） |
| 🟠 P1 | 短期（デバイス互換・安定性） |
| 🟡 P2 | 短期（音質・操作感） |
| 🔵 v2 | 中期（配信品質・評価基盤） |
| 🟣 v3 | 長期（AI音声変換） |

---

## Phase 0 — メタ整備（P0）

### T-001 🔴 Cargo.toml リポジトリURL修正 ✅
**ファイル**: `Cargo.toml:8`  
**問題**: `repository = "https://github.com/your-name/kuruvoice"` がプレースホルダのまま  
**作業**:
- [x] 実際のリポジトリURLに差し替える（または未公開なら行ごと削除）

**工数**: 5分

---

### T-002 🔴 README をコード実態に合わせる
**ファイル**: `README.md`  
**問題**: プリセット数・DSPチェーン順・必要 cargo バージョンなどが最新実装とずれている  
**作業**:
- [ ] プリセット一覧（`src/preset/presets.rs` から自動抽出して貼り付け）
- [ ] DSPチェーン順を `src/dsp/chain.rs` コメントから転記
- [ ] `cargo build --release` 手順と仮想ケーブル設定手順（VB-Cable / BlackHole）を追記
- [ ] Windows / macOS / Linux それぞれのセットアップ手順節を追加

**工数**: 2〜4h

---

## Phase 1 — デバイス互換・安定性（P1）

### T-003 🟠 サンプルフォーマット自動変換（i16/u16対応）
**ファイル**: `src/audio/engine.rs`  
**問題**: 入力ストリームが `build_input_stream::<f32>` のみ。i16/u16 デバイスで起動失敗または無音になる  
**作業**:
- [ ] `convert_to_f32<T: cpal::Sample>(src: &[T]) -> Vec<f32>` ヘルパーを `src/audio/device.rs` に追加
- [ ] `in_device.default_input_config()` の `SampleFormat` を match し、`f32 / i16 / u16` それぞれ対応コールバックを `build_input_stream` に渡す
- [ ] 出力側も同様に `f32 → SampleFormat` 変換ヘルパーを追加
- [ ] テスト: i16バッファ変換の単体テストを `tests/dsp_tests.rs` に追加

**参考コード（レビューより）**:
```rust
fn convert_samples<T: cpal::Sample>(input: &[T], output: &mut [f32]) {
    for (i, &s) in input.iter().enumerate() {
        output[i] = s.to_f32();
    }
}
```

**工数**: 1〜2日

---

### T-004 🟠 入出力サンプルレート不一致時のリサンプリング
**ファイル**: `src/audio/engine.rs:67-73`  
**問題**: レートが違うと `log::warn!` を出すだけで音程ずれが発生する  
**作業**:
- [ ] `Cargo.toml` に `rubato = "0.14"` を追加
- [ ] `Engine::start` 内で `in_rate != out_rate` のとき `rubato::FftFixedIn` リサンプラーを生成
- [ ] 処理スレッドで DSP 後に出力レートへ変換してから `out_prod` に push
- [ ] `AudioSection.sample_rate` 設定値を入力ストリームのレート要求に反映（現状無視されている）
- [ ] テスト: 44100→48000 変換で長さ・値のサニティチェック

**工数**: 2〜4日

---

### T-005 🟠 バッファサイズ設定反映
**ファイル**: `src/audio/engine.rs:75-85`, `src/config.rs:52`  
**問題**: `AudioSection.buffer_size` フィールドは定義されているが、`cpal::BufferSize::Default` が固定で使われており設定値が無視されている  
**作業**:
- [ ] `in_cfg.buffer_size` / `out_cfg.buffer_size` を `cpal::BufferSize::Fixed(config.audio.buffer_size as u32)` に変更
- [ ] 処理スレッドの `BLOCK` 定数を `config.audio.buffer_size` から動的に設定
- [ ] `config.example.toml` にコメント付きで推奨バッファサイズ例（128/256/512）を追記

**工数**: 半日

---

### T-006 🟠 起動エラーの GUI ダイアログ表示
**ファイル**: `src/app.rs`, `src/gui/mod.rs`  
**問題**: デバイスオープン失敗が `log::error!` のみで、GUIを開いているユーザーには何も見えない  
**作業**:
- [ ] `Engine::start` の `Err` を `App` 構造体の `Option<String>` エラーフィールドに格納
- [ ] `gui/dashboard.rs` でエラーフィールドが Some のとき egui の `Window::new("エラー")` でモーダル表示
- [ ] エラー内容に「仮想ケーブルが見つかりません → VB-Cable を確認してください」のような補足テキストを付ける
- [ ] テスト: 存在しないデバイス名を渡したとき `KuruError::DeviceError` が返ることを確認

**工数**: 半日〜1日

---

## Phase 2 — パラメータ更新品質（P2）

### T-007 🟡 DSP パラメータ差分更新 API（チェーン再構築廃止）
**ファイル**: `src/dsp/chain.rs`, `src/audio/engine.rs:151-153`  
**問題**: GUI スライダー操作のたびに `DspChain::from_config()` でチェーン全破棄・再構築が発生し、内部状態リセット→プチノイズが生じる  
**作業**:
- [ ] `AudioProcessor` トレイトに `fn update_params(&mut self, cfg: &AppConfig)` メソッドを追加（デフォルト実装: 何もしない）
- [ ] 各 DSP モジュール（`PitchFormant`, `Eq`, `Compressor`, `Limiter`, `DeEsser`, `AutoGain`, `NoiseGate`, `HarmonicEnhancer`, `Fluctuation`）に `update_params` 実装
- [ ] `DspChain::update_params(&mut self, cfg: &AppConfig)` を追加し、各モジュールに委譲
- [ ] `engine.rs` の `ParamUpdate::Config` ハンドラを `chain = DspChain::from_config(...)` から `chain.update_params(&cfg)` に切り替え
- [ ] テスト: パラメータ更新前後でチェーンの `names()` が変わらないことを確認

**工数**: 2〜4日

---

### T-008 🟡 パラメータ変化時の線形補間スムージング
**ファイル**: 各 `src/dsp/*.rs`（T-007 完了後に作業）  
**問題**: パラメータが瞬時に切り替わるため、大きな値変化（例: ピッチ±12半音を一気に変更）でクリックノイズが出る  
**作業**:
- [ ] `PitchFormant` でピッチ目標値への追従を `pitch = lerp(pitch, target, 0.05)` 形式（1ポールLPF）にする
- [ ] `Compressor` の threshold / ratio も同様にスムージング
- [ ] `Eq` のゲイン係数もスムージング（Biquad 係数直接補間ではなく目標ゲインdBを補間後に係数計算）
- [ ] スムージング時定数をテストしやすい定数 `PARAM_SMOOTH_ALPHA: f32 = 0.05` としてモジュール上部で定義
- [ ] テスト: ±12半音変化を与えたバッファに明示的なクリップ（±1.0超）が出ないこと

**工数**: 1〜2日

---

### T-009 🟡 バイパス切替のクロスフェード
**ファイル**: `src/dsp/chain.rs:49-55`, `src/audio/engine.rs`  
**問題**: `set_bypass(true/false)` が即時切替のため、処理済み信号→バイパス信号の急変でプチッとノイズが出る  
**作業**:
- [ ] `DspChain` に `bypass_fade: f32` フィールドを追加（0.0=フル処理、1.0=フルバイパス）
- [ ] `set_bypass` でフラグを立て、`process` 内でバッファを処理済み版と元バッファのブレンドで出力
- [ ] フェード速度: 約 20ms（`sample_rate * 0.02` サンプル）で 0→1 または 1→0 に変化
- [ ] テスト: フェード中に `bypass_fade` が単調増加/減少することを確認

**工数**: 1日

---

## Phase 2 — 配布整備（P2）

### T-010 🟡 Windows 向け配布パッケージ作成
**問題**: 現状 `cargo build` からのみ使用可能で、非エンジニアには敷居が高い  
**作業**:
- [ ] `cargo build --release` で生成した `kuruvoice.exe` を GitHub Releases に添付するワークフローを追加（`.github/workflows/release.yml`）
- [ ] `NSIS` または `WiX Toolset` で `.msi` インストーラを作成（VB-Cable ダウンロードページへのリンクを含むウィザード）
- [ ] コード署名手順をドキュメント化（自己署名でも可、手順を `docs/signing.md` に記載）
- [ ] インストーラに `config.example.toml` を同梱しデフォルト設定として配置

**工数**: 2〜5日

---

### T-011 🟡 macOS / Linux セットアップガイド整備
**作業**:
- [ ] macOS: BlackHole / Loopback との連携手順を `docs/setup_macos.md` に記載
- [ ] Linux: PipeWire / PulseAudio の仮想ソース作成コマンドを `docs/setup_linux.md` に記載
- [ ] macOS notarization の手順メモを追加（配布時に必要）

**工数**: 1〜2日

---

## Phase v2 — 評価基盤・音質強化（中期）

### T-012 🔵 音声評価コーパス整備
**ファイル**: `examples/voice_report.rs`, `src/eval/metrics.rs`  
**問題**: `AudioMetrics` は実装済みだが、標準テスト音声がなく数値比較ができない  
**作業**:
- [ ] `tests/corpus/` ディレクトリを作成し、以下を収録（またはダウンロードスクリプトを作成）
  - 男性低音・男性高音・女性低音・女性高音の各5秒サンプル（自録または CC0素材）
  - 早口語・サ行連続・雑音混入（ホワイトノイズ混合）サンプル
- [ ] `examples/voice_report.rs` を拡張してコーパス全ファイルに対してバッチ評価し CSV 出力
- [ ] 合格閾値をテストとして定義: ドロップアウト率 < 0.1%、クリップ率 < 0.01%、P95遅延 < 100ms

**工数**: 3〜5日

---

### T-013 🔵 遅延・ドロップアウト計測の自動化
**ファイル**: `src/eval/metrics.rs`, `examples/golden_report.rs`  
**問題**: `AudioMetrics` の `latency_p50/p95` 収集はあるが、CI で自動比較される仕組みがない  
**作業**:
- [ ] `examples/golden_report.rs` を `cargo bench` 相当の bench テストに変換し、数値が閾値を超えたら非ゼロ終了
- [ ] GitHub Actions ワークフローに `cargo run --example golden_report` ステップを追加
- [ ] 結果を `target/eval_report.json` に書き出してアーティファクトに保存

**工数**: 2〜3日

---

### T-014 🔵 プリセット調整ウィザード（GUI内）
**ファイル**: `src/gui/dashboard.rs`, `src/preset/presets.rs`  
**問題**: プリセットは固定12種のみで、ユーザーが微調整して保存する手段がない  
**作業**:
- [ ] GUI に「名前を付けて保存」ボタンを追加（`egui::TextEdit` で名前入力 → `AppConfig::save` を呼ぶ）
- [ ] ユーザー定義プリセットを `~/.config/kuruvoice/presets/` に保存・読み込み
- [ ] プリセット選択ドロップダウンを組み込みプリセット＋ユーザープリセットの混合リストに拡張
- [ ] テスト: 保存→読み込みで `AppConfig` が PartialEq で一致すること

**工数**: 2〜3日

---

### T-015 🔵 OBS / Discord 連携ガイドと UI 表示
**作業**:
- [ ] `docs/obs_integration.md`: OBS の「音声入力キャプチャ」で KuruVoice 出力（仮想ケーブル）を選ぶ手順
- [ ] `docs/discord_integration.md`: Discord のマイク設定で仮想ケーブルを選ぶ手順
- [ ] GUI に「接続状態ヒント」パネル: 現在の出力デバイス名を表示し "OBS/Discord ではこのデバイスを選択してください" とガイドを表示

**工数**: 1〜2日

---

## Phase v3 — AI 音声変換（長期）

### T-016 🟣 ONNX ランタイム統合基盤
**ファイル**: `Cargo.toml`, `src/audio/engine.rs`  
**前提**: T-007 完了（チェーン再構築廃止）  
**作業**:
- [ ] `Cargo.toml` に `ort = "2"` （ONNX Runtime Rust バインディング）を追加
- [ ] `src/ai/mod.rs` を新規作成し `OnnxInferenceBlock` 構造体を定義（`AudioProcessor` トレイト実装）
- [ ] 推論は別スレッドで非同期実行し、PCMデータを `crossbeam-channel` でやり取り（RTコールバックをブロックしない）
- [ ] `OnnxInferenceBlock` を `DspChain` の末尾（リミッター前）に挿入できる設計にする
- [ ] テスト: ダミーモデル（identity変換）を通して遅延・精度の基準値を取得

**工数**: 1〜2週間

---

### T-017 🟣 自声補正モデルの学習・組み込み
**前提**: T-016 完了  
**作業**:
- [ ] 学習データ収集スクリプト作成（元声 → 目標声のペアを `tests/corpus/` から自動生成）
- [ ] 軽量エンコーダ・デコーダ（例: 1D Conv + GRU）を PyTorch で学習し ONNX エクスポート
- [ ] モデルを `assets/models/voice_enhance_v1.onnx` として同梱
- [ ] `AudioSection` に `ai_model_path: Option<String>` フィールドを追加して切り替え可能にする
- [ ] なりすまし防止: 推論時に声紋類似度スコアを算出し、既知の特定人物へ一定以上類似する場合は処理を拒否してGUI警告を出す

**工数**: 4〜8週間

---

### T-018 🟣 プラットフォーム高度化（ASIO / CoreAudio 対応）
**作業**:
- [ ] Windows: CPAL の `asio` feature を有効化し、`features = ["asio"]` フラグで条件ビルド
- [ ] ASIO SDK の取得方法と `CPAL_ASIO_DIR` 環境変数設定を `docs/asio_setup.md` に記載
- [ ] macOS: CoreAudio 経由のバッファサイズ最小化（64サンプル以下）を検証
- [ ] Linux: JACK バックエンド対応を検討（cpal の `jack` feature）

**工数**: 1〜2週間

---

## タスク一覧（優先順）

| # | タイトル | 優先度 | 工数目安 | 依存 |
|---|---------|--------|---------|------|
| # | タイトル | 優先度 | 工数目安 | 依存 | 状態 |
|---|---------|--------|---------|------|------|
| T-001 | Cargo.toml URL修正 | 🔴 P0 | 5分 | — | ✅ done |
| T-002 | README更新 | 🔴 P0 | 2〜4h | — | ✅ done |
| T-003 | サンプルフォーマット対応 | 🟠 P1 | 1〜2日 | — | ✅ done |
| T-004 | リサンプリング統合 | 🟠 P1 | 2〜4日 | — | ✅ done |
| T-005 | バッファサイズ設定反映 | 🟠 P1 | 半日 | — | ✅ done |
| T-006 | 起動エラーGUI表示 | 🟠 P1 | 半日〜1日 | — | ✅ done |
| T-007 | DSP差分更新API | 🟡 P2 | 2〜4日 | — | ✅ done |
| T-008 | パラメータスムージング | 🟡 P2 | 1〜2日 | T-007 | ✅ done |
| T-009 | バイパスクロスフェード | 🟡 P2 | 1日 | T-007 | ✅ done |
| T-010 | Windows/CI配布パッケージ | 🟡 P2 | 2〜5日 | T-001 | ✅ done |
| T-011 | macOS/Linux セットアップガイド | 🟡 P2 | 1〜2日 | — | ✅ done |
| T-012 | 音声評価コーパス | 🔵 v2 | 3〜5日 | — | ✅ done |
| T-013 | 遅延計測自動化 / 閾値チェック | 🔵 v2 | 2〜3日 | T-012 | ✅ done |
| T-014 | プリセット調整ウィザード | 🔵 v2 | 2〜3日 | — | ✅ done |
| T-015 | OBS/Discord連携ガイド | 🔵 v2 | 1〜2日 | T-010 | ✅ done |
| T-016 | ONNXランタイム統合基盤 | 🟣 v3 | 1〜2週 | T-007 | ✅ done (stub) |
| T-017 | 自声補正モデル学習スクリプト | 🟣 v3 | 4〜8週 | T-016 | ✅ done (script) |
| T-018 | ASIO/CoreAudio対応 | 🟣 v3 | 1〜2週 | — | ✅ done |

---

## 参考: 修正が必要な主要ファイルマップ

```
src/
├── audio/
│   ├── engine.rs        ← T-003, T-004, T-005, T-007, T-016
│   └── device.rs        ← T-003
├── dsp/
│   ├── chain.rs         ← T-007, T-009
│   ├── pitch_formant.rs ← T-008
│   ├── compressor.rs    ← T-008
│   ├── eq.rs            ← T-008
│   └── *.rs             ← T-007（update_params追加）
├── gui/
│   ├── dashboard.rs     ← T-006, T-014, T-015
│   └── mod.rs           ← T-006
├── config.rs            ← T-004, T-005, T-017
├── eval/
│   └── metrics.rs       ← T-013
Cargo.toml               ← T-001, T-004, T-016
README.md                ← T-002
```
