# KuruVoice 商用化 タスク分解バックログ

提案（KuruVoice Commercial Edition v1 / Pro v2 / Platform v3）を、**現状コードに紐づけた
着手しやすいタスク**に分解したもの。各タスクは独立して着手・レビューできる粒度。

- 関連: [virtual_audio_design.md](virtual_audio_design.md) / [design.md](design.md) / [preset.md](preset.md)
- 方針（提案より）: **「綺麗に入れる → 綺麗に変える → 綺麗に出す」**。AI を先に入れず、
  入力補正と評価基盤を先に固める。

---

## 0. 現状スナップショット（2026-06-04 時点）

| 区分 | 実装済み | 未実装 / 弱い |
|---|---|---|
| DSPチェーン | DC→denoise→gate→AGC→pitch/formant→EQ→harmonic→**de-esser**→comp→limiter | plosive / de-breath |
| ピッチ/フォルマント | 位相ボコーダ + ケプストラム包絡 + gather型ピッチアップ + formant-follow(0.5) | follow量がハードコード |
| ノイズ | spectral subtraction (`denoise.rs`) + VAD連動ノイズ推定 | RNNoise級の抑制 / 残響・反射対策 |
| プリセット | Neutral/Soft/Bright Feminine, Young Neutral, Ikemen 系, Bright High, Deep Cool | 評価ベースの再チューニング / 動的プリセット |
| 声質マクロ | `voice_character`（明瞭/かわいい/かっこいい/怖い レーダー） | F0等の自動分析と連動なし |
| 仮想マイク | Win=ケーブル自動検出 / Linux=pactl 仮想mic | 実機検証 / 出力自動選択 / OBSフィルタ |
| 評価 | `eval/metrics`（peak/rms/clip率/noise floor/percentile遅延） + `examples/golden_report.rs` | AB比較 / レポート生成 / ベンチ統合 |
| AI(VC) | なし | ONNX Runtime / Neural VC 一式（v1 Phase3〜） |
| 配布/安全 | LICENSE/SECURITY/CI | モデル署名 / manifest / consent / インストーラ |
| 構成 | 単一 crate | workspace 分割（core/dsp/ai/...） |

> 結論: **v1 Phase 0–1 はほぼ完了**。次の山は **評価基盤（E0）→ 入力補正（E1）→ 声質の作り込み
> （E2/E3）**。AI（E5）はその後。

---

## 1. タスクの読み方

- **ID**: `KV-<エピック>-<番号>`
- **規模**: S=半日 / M=1〜2日 / L=3〜5日 / XL=1〜2週+
- **優先度**: P0=今やる / P1=次 / P2=後
- **DoD（共通の完了条件）**:
  1. `cargo fmt --check` / `cargo clippy --all-targets -- -D warnings` / `cargo test` 全通過
  2. 音に関わる変更は `--example voice_report` か golden 差分で**数値**を提示
  3. 既定挙動を壊さない（新機能は既定OFF or 後方互換）

---

## 2. エピック一覧

| エピック | 内容 | 提案Phase | 状態 |
|---|---|---|---|
| **E0** 評価基盤 | golden音声・AB比較・ベンチ・レポート | v1 P2(前倒しP0) | 一部完了(golden/metrics) |
| **E1** 入力補正 | high-pass独立 / AGC / plosive / de-breath / denoise強化 | v1 P1 | 一部完了(AGC/VAD denoise) |
| **E2** DSP声質 | harmonic enhancer / spectral tilt / 上げ品質 / follow可変 | v1 P1 | 一部完了(harmonic/gather) |
| **E3** プリセット&診断 | 再チューニング / VoiceAnalyzer / キャリブ / 設定保存 | v1 P0/P1 | 一部 |
| **E4** 仮想マイク・配信 | Win UX / Linux検証 / OBS相性 | v1 P1 | 一部 |
| **E5** AI Fast | ONNX統合 / 特徴抽出 / 軽量VC / fallback | v1 P1〜Phase3 | 未 |
| **E6** AI HQ | 高品質vocoder / offline / 量子化 / GPU | v1 Phase4 | 未 |
| **E7** 配布Hardening | 署名 / manifest / consent / installer / SBOM | v1 Phase5 | 未 |
| **E8** v2 Pro | Voice Wizard / 動的プリセット / 録音AB / VST3 | v2 | 未 |
| **E9** v3 Platform | SDK / Model Store / OBS plugin / Unity / safety | v3 | 未 |

---

## 3. タスク詳細（v1：E0〜E4 を細かく）

### E0 評価基盤（まず「良くなったか」を測れるように）

| ID | 規模 | 優先 | タスク / 受け入れ条件 | 触る場所 | 依存 |
|---|---|---|---|---|---|
| KV-EVAL-1 | M | P0 | ✅ **golden音声ハーネス**。`tests/audio/` に male_low/male_mid/female/noisy_room/keyboard の WAV を用意し、各プリセットでオフライン変換→`AudioMetrics`(clip率/noise floor/RMS/peak) を算出する `examples/golden_report.rs`。基準値を JSON 保存し回帰検知。 | `examples/`, `eval/` | EVALメトリクス(済) |
| KV-EVAL-2 | S | P0 | **遅延ベンチ拡張**。`perf` 例に `eval::percentile_latency_ms` で p50/p95/max を出力。モード別（DSP only / 将来AI）に分岐枠を用意。 | `examples/perf.rs`, `eval` | - |
| KV-EVAL-3 | M | P1 | **AB比較出力**。原音 / DSP変換後（将来 AI Fast/HQ）を WAV 書き出し + メトリクス比較表を Markdown 生成する CLI (`--ab-test in.wav`)。 | `cli`, `eval/ab_test.rs`(新) | EVAL-1 |
| KV-EVAL-4 | S | P1 | **レポート自動生成**。`voice_report.md` / `latency_report.json` を吐く `eval/report.rs`。CI で artifact 化。 | `eval/report.rs`(新), `.github` | EVAL-1/2 |

### E1 入力補正（汚い入力を AI/DSP 前に整える）

| ID | 規模 | 優先 | タスク / 受け入れ条件 | 触る場所 | 依存 |
|---|---|---|---|---|---|
| KV-IN-1 | S | P0 | **High-pass を独立ステージ化**（現状 EQ 内）。`[input] high_pass_hz` 追加、チェーン先頭側へ。EQ の HPF と二重化しない整理。 | `dsp/`, `config.rs`, `chain.rs` | - |
| KV-IN-2 | M | P0 | ✅ **Auto Gain (AGC)**。目標 RMS/LUFS へ緩やか追従、無音時は据え置き。過大入力時はクリップ前に抑制。 | `dsp/auto_gain.rs`, `config`, `chain` | - |
| KV-IN-3 | M | P1 | **Plosive control**。低域の破裂トランジェント検出→短時間ダッキング。 | `dsp/plosive.rs`(新) | KV-IN-1 |
| KV-IN-4 | M | P1 | **De-breath**。息区間（高域寄り・低周期性）を検出して減衰。ゲートと連動。 | `dsp/debreath.rs`(新) | - |
| KV-IN-5 | L | P1 | ✅/継続 **Denoise 強化**。VAD連動で持続音の誤抑制は対策済み。次は RNNoise系(`nnnoiseless`等)導入の調査・PoC、musical noise/残響低減。 | `dsp/denoise.rs`, 調査メモ | EVAL-1 |

### E2 DSP 声質（女性・中性の「細さ/金属感/こもり」対策）

| ID | 規模 | 優先 | タスク / 受け入れ条件 | 触る場所 | 依存 |
|---|---|---|---|---|---|
| KV-DSP-1 | M | P0 | ✅ **Harmonic enhancer**。上げ時に痩せる倍音を補う（軽い倍音生成＋帯域制御）。女性/中性で「太さ・芯」を回復。golden で高域充実を確認。 | `dsp/harmonic.rs` | EVAL-1 |
| KV-DSP-2 | S | P1 | **Spectral tilt** を独立パラメータ化（明るさ/暗さ）。`voice_character` の brightness と接続。 | `dsp/`, `config`, `voice_character` | - |
| KV-DSP-3 | L | P0 | ✅ **ピッチアップ品質改善**。`pitch_formant` のピッチアップを target-bin gather 型に変更し、女性域(+5〜+7)の金属感/コム感を低減（golden のスペクトル平滑度で評価）。 | `dsp/pitch_formant.rs` | EVAL-1 |
| KV-DSP-4 | S | P1 | **formant-follow を可変化**。現状ハードコード(0.5)を `config.voice.formant_follow` + GUI スライダーに。 | `pitch_formant`, `config`, `gui` | - |
| KV-DSP-5 | M | P1 | ✅ **1/f ゆらぎモード**。ピンク(1/f)ノイズ駆動の微小ピッチ揺れ+音量揺れで機械感を消し自然化。`dsp/fluctuation.rs`、`[fluctuation]`、GUI 節、テスト3件。 | `dsp/fluctuation.rs`, `config`, `gui` | - |

### E3 プリセット & 診断

| ID | 規模 | 優先 | タスク / 受け入れ条件 | 触る場所 | 依存 |
|---|---|---|---|---|---|
| KV-PRE-1 | M | P0 | **女性/中性プリセット再チューニング**。Neutral Clean / Soft Feminine / Bright Feminine を golden で数値確認しつつ調整（刺さり・こもり・痩せ）。 | `preset/presets.rs` | EVAL-1, DSP-1/3 |
| KV-PRE-2 | M | P1 | **VoiceAnalyzer**。median F0 / loudness / noise floor / brightness / sibilance / clip率 を推定（`VoiceAnalysisResult`）。 | `voice/analyzer.rs`(新) | EVAL(metrics) |
| KV-PRE-3 | L | P1 | **キャリブレーション・ウィザード**。10秒録音→分析→推奨プリセット提示（GUI ウィザード）。 | `gui/calibration.rs`(新), `voice` | PRE-2 |
| KV-PRE-4 | M | P2 | **動的プリセット**（`DynamicVoiceProfile`）。分析値でプリセットを自動微調整。v2 土台。 | `voice/profile.rs`(新) | PRE-2 |
| KV-PRE-5 | S | P0 | **設定の保存/復元拡充**。`voice_character` / `denoise` / 新パラメータも TOML 保存対象に（現在 GUI 状態が一部未保存）。 | `config`, `gui/dashboard.rs` | - |

### E4 仮想マイク・配信

| ID | 規模 | 優先 | タスク / 受け入れ条件 | 触る場所 | 依存 |
|---|---|---|---|---|---|
| KV-VM-1 | S | P1 | **Win ルーティング UX**。受け側マイク名のコピー、設定中バッジ、未検出時の導線改善（検出は実装済）。 | `gui`, `audio/virtual_cable.rs` | - |
| KV-VM-2 | M | P1 | **Linux 仮想マイク実機検証**＋出力自動選択（`KuruVoice_Sink`）。失敗時フォールバック。 | `audio/virtual_mic.rs`, `gui` | - |
| KV-VM-3 | S | P1 | **配信相性チェック**。OBS/Discord/Zoom で 48kHz 同期・音切れの手順書＆簡易自己診断。 | `docs/`, `eval` | EVAL-2 |

### E5 AI Fast（女性・中性の本命。E0〜E2 安定後）

| ID | 規模 | 優先 | タスク / 受け入れ条件 | 触る場所 | 依存 |
|---|---|---|---|---|---|
| KV-AI-1 | M | P1 | **AI 骨子**。`ai/` に `InferenceEngine` trait・`ModelManifest`・`fallback` の枠と署名検証 stub。音声は通さず型と読込のみ。 | `ai/`(新) | - |
| KV-AI-2 | L | P1 | **ONNX Runtime 統合**（`ort` crate）。CPU 推論でダミーモデル疎通、フレーム I/O 整合。 | `ai/onnx_engine.rs` | AI-1 |
| KV-AI-3 | L | P1 | **特徴抽出 + 遅延管理**。mel/F0 抽出、lookahead、ブロック整合。p95 遅延を eval で計測。 | `ai/feature_extractor.rs` | AI-2, EVAL-2 |
| KV-AI-4 | XL | P1 | **軽量 VC モデル統合**（QuickVC/VITS系を選定→ONNX化→接続）。Fast モードで動作。 | `ai/`, `models/` | AI-3 |
| KV-AI-5 | M | P1 | **モード切替 & フォールバック**（DSP/AI Fast/AI HQ/Offline、重負荷時に自動降格）。 | `ai/engine.rs`, `app` | AI-2 |
| KV-AI-6 | XL | P1 | **Neutral/Soft Feminine モデル作成**（JVS/JSUT 等の同意データ。実在人物クローン禁止）。 | 学習基盤(別) | AI-4 |

### E6 AI HQ / E7 配布（概要のみ。詳細は着手前に展開）

- **E6**: 高品質 vocoder、Offline Render、モデル量子化、GPU(CUDA/DirectML/CoreML) backend。
- **E7**: モデル署名・manifest 検証本実装、consent フラグ、インストーラ(MSI/pkg)、自動更新、SBOM、クラッシュレポート、モデルライセンス/利用規約。

---

## 4. v2（Pro）/ v3（Platform）エピック（粗粒度・着手前に詳細化）

### v2: KuruVoice Pro
- **E8-1 Voice Wizard**（初回診断→推奨）｜依存 PRE-2/3
- **E8-2 動的プリセット製品化**｜依存 PRE-4
- **E8-3 録音・AB比較 UI**（wav/flac/mp3、推定MOS）｜依存 EVAL-3
- **E8-4 仮想マイク標準化**（全OS）｜依存 E4
- **E8-5 VST3 プラグイン**（まず録音音声向け Effect）
- **E8-6 安全設計**（実在人物プリセット禁止、署名バッジ、ローカル処理表示）
- 完成条件: OBS/Discord/Zoom 常用、Neutral/Soft Feminine が実用、AI Fast p95≤80ms、30分音切れ0〜1、VST3 で変換可。

### v3: KuruVoice Platform
- **E9-1 SDK 化**（Rust + C ABI、`KuruVoiceEngine`）
- **E9-2 Model Manifest v3 + Model Store 基盤**（署名/更新/ライセンス）
- **E9-3 Voice Profile Creator**（同意必須・他人音声アップロード禁止）
- **E9-4 OBS Plugin / Unity Bridge**
- **E9-5 Safety Scanner / Watermark**
- **E9-6 Team Console / Cloud(任意・オプトイン)**
- 完成条件: 他アプリから Engine 利用可、公式/非公式モデルを安全に区別、SDK/VST3/Unity が揃う、同意済み声質作成フロー。

---

## 5. 推奨着手順（Sprint 0 = 次の5タスク）

直近の課題（女性声・中性声が弱い）を、**測って→直して→測る**で潰す順序:

1. **KV-EVAL-1**（golden音声ハーネス）← まず良し悪しを数値化
2. **KV-DSP-3**（ピッチアップ品質＝女性域の金属感）← 体感の主因
3. **KV-DSP-1**（harmonic enhancer＝細さ/芯の回復）
4. **KV-PRE-1**（女性/中性プリセット再チューニング）← 1〜3 を反映
5. **KV-IN-2**（Auto Gain）＋ **KV-PRE-5**（設定保存）← 実用性の底上げ

この 5 つで「女性・中性が実用」に近づき、以降 E1 残り → E5(AI) へ。

---

## 6. リスク・前提

- **DSP の限界**: 大幅クロスジェンダー（自然な女性声）は DSP 単体では頭打ち。最終品質は **AI VC（E5/E6）** が必要。E0〜E2 はその「前処理品質」を上げる投資でもある。
- **聴感評価**: 数値（clip/noise/spectral）に加え、最終判断は実際の試聴（MOS/AB）。CI では数値回帰のみ担保。
- **法務/倫理**: 実在人物の無断クローンは全フェーズで禁止。AI モデルは同意済みデータのみ。
- **構成**: E5 着手前に **workspace 分割**（`kuruvoice-core/dsp/ai/audio/eval/...`）を推奨（AI 依存をコアから隔離）。→ 任意タスク **KV-ARCH-1（M）**。
