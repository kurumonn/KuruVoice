<div align="center">

# 🎙 KuruVoice

**軽量・低遅延のリアルタイム「イケメン」ボイスチェンジャー（Rust 製）**

マイクの声を低遅延で加工し、「整った低音・爽やか・聞き取りやすい声」に補正します。
配信(OBS) / 通話(Discord) / VRChat などにそのまま使えます。

[導入方法](#-導入方法クイックスタート) ・ [使い方](#-使い方) ・ [プリセット](#-プリセット) ・ [配信連携](#-配信連携obs--discord--vrchat) ・ [仕組み](#-仕組み) ・ [配布](#-配布) ・ [開発](#-開発)

</div>

> [!NOTE]
> KuruVoice は **AI で別人の声にするツールではありません**。DSP（信号処理）で
> **自分の声を整える**ためのものです。なりすまし等の利用は禁止しています（[セーフティ](#-セーフティ)）。

---

## ✨ 特長

| | |
|---|---|
| 🎚 **ダッシュボード GUI** | プリセット選択・スライダー調整・入出力メーター・ワンクリック ON/OFF |
| 🔉 **DSP チェーン 11 段** | DC カット → **ノイズ低減** → ノイズゲート → **自動ゲイン(AGC)** → ピッチ/フォルマント(位相ボコーダ) → **1/f ゆらぎ** → EQ → **ハーモニクス** → **De-esser** → コンプ → **リミッター** |
| 🎛 **12 プリセット** | Natural Low / Ikemen Soft / Ikemen Deep / Narrator / Clear Streaming / Radio Voice / Bright High / Deep Cool / **Warm Narrator** / **Live Streamer** / **Anime High** / **Cool Whisper** |
| 🎨 **声の印象グラフ** | 「明瞭さ・かわいさ・かっこよさ・怖さ」をレーダーチャートでドラッグ調整（専門用語不要） |
| 🎚 **高品質ピッチ/フォルマント** | 位相ボコーダ＋ケプストラム包絡で ±6 半音でも自然。声の太さをピッチと独立に調整 |
| 🌿 **1/f ゆらぎモード** | ピンク(1/f)ノイズで微小なピッチ/音量揺れを加え、機械っぽさを消して自然・心地よく |
| ⚡ **低遅延・軽量** | GPU 不要。1 コアの数 % で動作（[実測](#-性能と効果の実測)） |
| 📝 **TOML 設定** | GUI のツマミと 1:1 対応。保存・共有が簡単 |
| 🖥 **CLI も完備** | `--list-devices` / `--preset` / `--record-test` など |

対応 OS: **Windows 10/11**（優先）/ Linux / macOS

---

## 🚀 導入方法（クイックスタート）

### 1. 配布版を使う（Windows 推奨）

Windows では GitHub Releases からインストーラーまたは zip をダウンロードして使えます。

1. [Releases](https://github.com/kurumonn/KuruVoice/releases) を開く
2. 最新版の `KuruVoiceSetup-vX.Y.Z-windows-x64.exe` をダウンロード
3. インストーラーを実行して KuruVoice を起動

zip で使いたい場合は `KuruVoice-vX.Y.Z-windows-x64.zip` を展開し、`kuruvoice.exe` を実行してください。

> [!IMPORTANT]
> 初期配布の exe は未署名のため、Windows SmartScreen で警告が出る場合があります。
> ファイルの確認用に Release Assets の `.sha256` も併せて公開しています。

### 2. ソースからビルドする

開発版を試す場合や Linux / macOS で使う場合は、Rust ツールチェインを入れてビルドします。

Rust（1.75 以上）が必要です。未インストールなら [rustup](https://rustup.rs/) から入れます。

<details>
<summary><b>Windows</b></summary>

1. [https://rustup.rs/](https://rustup.rs/) から `rustup-init.exe` を実行（既定の選択でOK）。
2. インストール時に **「Visual Studio C++ Build Tools」** を求められたら一緒に入れる
   （`cl.exe` リンカが必要なため）。
3. ターミナルを開き直して `cargo --version` が表示されればOK。
</details>

<details>
<summary><b>Linux (Ubuntu/Debian)</b></summary>

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# オーディオ開発ヘッダが必要
sudo apt-get update && sudo apt-get install -y libasound2-dev pkg-config
```
</details>

<details>
<summary><b>macOS</b></summary>

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
xcode-select --install   # 未導入なら
```
</details>

#### 取得してビルド

```bash
git clone https://github.com/kurumonn/KuruVoice.git
cd KuruVoice
cargo build --release
```

> 初回ビルドは依存（egui / cpal など）のコンパイルで数分かかります。2 回目以降は数秒です。

#### 起動

```bash
# ① まず使えるデバイスを確認
cargo run --release -- --list-devices

# ② ダッシュボード GUI を起動（引数なし）
cargo run --release
```

GUI が開いたら **左で入力（マイク）と出力を選び → 上部の「▶ 開始」** を押すだけです。

> 配信・通話アプリに乗せたい場合は [配信連携](#-配信連携obs--discord--vrchat) を参照（仮想オーディオデバイスが必要です）。

---

## 🎮 使い方

### GUI

1. **左ペイン**でマイク（入力）と出力先を選ぶ
2. 上部の **「▶ 開始」** を押す
3. 中央の **プリセット**ボタンで雰囲気を決める
4. **スライダー**で微調整（実行中でも即反映）
5. **「バイパス」**トグルで加工前後を聴き比べ
6. **設定ファイル**欄で TOML 保存／読込

> 実行中はデバイスを変えられません。変えるときは一度 **「■ 停止」** してから。

### CLI

```bash
cargo run --release -- --list-devices           # 入出力デバイス一覧
cargo run --release -- --preset ikemen_deep      # プリセット指定で GUI 起動
cargo run --release -- --config config.toml      # 設定ファイル指定
cargo run --release -- --no-gui --preset narrator # GUI なし常駐
cargo run --release -- --record-test 10          # 10 秒テスト録音(前後 WAV 出力)
cargo run --release -- --bypass                  # 加工なしで起動
```

設定は `config.example.toml` をコピーして編集してください（各キーは GUI のツマミと対応）。
詳細は [docs/usage.md](docs/usage.md)。

---

## 🎛 プリセット

| プリセット | ピッチ | 印象 | 用途 |
|---|---|---|---|
| **Natural Low** | -2 半音 | 自然に少し低く | 普段使い |
| **Ikemen Soft** | -3 半音 | 爽やか・柔らかい低音 | 配信・通話 |
| **Ikemen Deep** | -4 半音 | 深く落ち着いた声 | 朗読・ナレ |
| **Narrator** | -2 半音 | 聞き取りやすい | 解説動画 |
| **Clear Streaming** | -1 半音 | 明瞭感重視 | 配信 |
| **Radio Voice** | -3 半音 | ラジオ風の太い声 | 演出 |
| **Bright High** | +6 半音 | 明るく軽い高めの声 | 高い声を作りたい人 |
| **Deep Cool** | -6 半音 | 低く渋い太めの声 | 低い声を作りたい人 |
| **Warm Narrator** | -1 半音 | 温かみのある語り声 | ポッドキャスト・朗読 |
| **Live Streamer** | 0 半音 | そのままで聞き取りやすく | ゲーム配信 |
| **Anime High** | +4 半音 | 軽く明るいアニメ声 | キャラクター演技 |
| **Cool Whisper** | -2 半音 | 囁き系低めの声 | ASMR・雑談 |

> ピッチ/フォルマントは**位相ボコーダ＋ケプストラム包絡**で処理するため、±6 半音の大きな変化でも
> 比較的自然です（フォルマント＝声の太さをピッチと独立に調整可能）。なお**特定の人物の声の再現は
> 行いません**（[セーフティ](#-セーフティ)）。

詳細とパラメータは [docs/preset.md](docs/preset.md)。

---

## 📊 性能と効果の実測

同梱のベンチで CPU 負荷と「声がどれだけ変わるか」を計測できます。

```bash
cargo run --release --example perf          # E2E 負荷テスト
cargo run --release --example voice_report  # 声の変化量（ピッチ・音色）を数値化
cargo run --release --example golden_report -- --baseline docs/golden_baseline.json
```

`golden_report` は `tests/audio/*.wav` を各プリセットに通して peak / RMS / clip率 /
noise floor を JSON 出力し、保存済み baseline と比較します。初回 baseline 作成:

```bash
cargo run --release --example golden_report -- --generate-fixtures --write-baseline docs/golden_baseline.json
```

### 負荷（実測：Windows・48kHz・256 サンプル/ブロック）

1 ブロックの締切は **5.33 ms**。全段有効（位相ボコーダ込み）でも処理は **0.05 ms 前後**＝
締切の 1% 未満で、バッファ落ち（アンダーラン）は発生しません。

| プリセット | 平均/ブロック | 最大/ブロック | CPU(1コア) | 実時間比 |
|---|---|---|---|---|
| 全段(ikemen_soft) | 0.046 ms | 0.29 ms | **0.87%** | **115×** |
| ikemen_deep | 0.046 ms | 0.13 ms | 0.86% | 116× |
| radio_voice | 0.046 ms | 0.23 ms | 0.87% | 115× |
| bypass | ~0 ms | 0.0003 ms | 0.00% | 194,000× |

→ **CPU 負荷は 1 コアの 1% 未満**（高品質ピッチ/フォルマント込み）。体感遅延はバッファ
（256 sample ≒ 5.3 ms）＋位相ボコーダの窓（約 16 ms、ピッチ/フォルマント使用時のみ）。GPU 不要。

### 声の変化量（実測：合成音声 f0=130Hz を各プリセットで加工）

| プリセット | ピッチ | 出力 f0 | 重心変化 | 印象 |
|---|---|---|---|---|
| natural_low | -1.99 半音 | 116 Hz | -7% | 自然に少し低く |
| ikemen_soft | -3.01 半音 | 109 Hz | -3% | 柔らかい低音 |
| ikemen_deep | -4.00 半音 | 103 Hz | -23% | 深く太い |
| narrator | -1.99 半音 | 116 Hz | -2% | 明瞭・通る声 |
| clear_streaming | -1.00 半音 | 123 Hz | +5% | 明るく明瞭 |
| radio_voice | -3.01 半音 | 109 Hz | -13% | 太いラジオ声 |
| **bright_high** | **+5.99 半音** | **184 Hz** | **+73%** | 明るく高い声 |
| **deep_cool** | **-6.01 半音** | **92 Hz** | **-27%** | 低く渋い声 |

→ **ピッチは設計値どおり -6〜+6 半音を正確に実現**（130Hz → 92〜184Hz）。位相ボコーダ＋
ケプストラム包絡で、さらに**フォルマント・フォロー**（ピッチに合わせて声の共鳴も半分追従）により
**高い声は明るく・低い声は太く**出ます（高低のメリハリ）。フォルマントは手動でも独立調整可能。

> 実測値は環境で変わるので、上記コマンドでご確認ください。

---

## 🔌 配信連携（OBS / Discord / VRChat）

KuruVoice の加工音を他アプリへ送るには **仮想オーディオデバイス**を経由します。

> 🎧 KuruVoice は起動時に **VB-CABLE / VoiceMeeter / BlackHole を自動検出**し、左ペインの
> 「仮想マイク」からワンクリックで出力先に設定できます（受け側で選ぶマイク名も表示）。
> 将来的には専用の仮想マイクを内蔵予定です（[設計](docs/virtual_audio_design.md)）。

1. 仮想デバイスを導入
   - Windows: [VB-CABLE](https://vb-audio.com/Cable/) または VoiceMeeter
   - macOS: [BlackHole](https://existential.audio/blackhole/)
   - **Linux: 左ペインの「🎙 KuruVoice 仮想マイクを作成」で外部ソフト無しに `KuruVoice_Mic` を生成可能**（PipeWire/PulseAudio）
2. KuruVoice の **入力 = 実マイク** / **出力 = 仮想デバイス** に設定して「▶ 開始」
3. 受け側アプリのマイク入力を **仮想デバイスの出力**に設定
   - OBS: 「音声入力キャプチャ」→ `CABLE Output`
   - Discord: 設定 → 音声・ビデオ → 入力 `CABLE Output`
   - VRChat: Settings → Audio → Microphone `CABLE Output`

手順詳細は [docs/obs_integration.md](docs/obs_integration.md) と [docs/discord_integration.md](docs/discord_integration.md)。

---

## 🧠 仕組み

```text
マイク → 入力(cpal) → リングバッファ → DSP スレッド → リングバッファ → 出力(cpal) → 仮想デバイス/OBS
```

処理順序（リミッターは必ず最終段＝音割れ防止）:

```text
DC カット
  → ノイズ低減 (スペクトル減算)
  → ノイズゲート
  → 自動ゲイン (AGC)
  → ピッチ/フォルマント (STFT 位相ボコーダ + ケプストラム包絡)
  → 1/f ゆらぎ (ピンクノイズ変調)
  → EQ (パラメトリック多バンド)
  → ハーモニクスエンハンサー
  → De-esser (動的歯擦音低減)
  → コンプレッサー
  → リミッター  ← 最終安全装置（必ず最後）
```

ピッチとフォルマントは STFT 位相ボコーダ＋ケプストラム包絡で処理し、ピッチを大きく変えても
フォルマント（声の太さ）を保ったまま／独立に動かせます。

オーディオコールバックはバッファの受け渡しだけを行い、重い処理は専用スレッドで実行する設計です
（低遅延と安定性の両立）。アーキテクチャ詳細は [docs/design.md](docs/design.md)。

---

## 📦 配布

タグ `v*` を push すると GitHub Actions が Windows / Linux / macOS 向けにビルドし、GitHub Release に配布物をアップロードします。

Windows 向けには次の2種類を公開します。

- `KuruVoiceSetup-vX.Y.Z-windows-x64.exe`: 一般ユーザー向けインストーラー
- `KuruVoice-vX.Y.Z-windows-x64.zip`: 展開してすぐ使える zip

ローカルで Windows 配布物を作る場合:

```powershell
.\scripts\package-windows.ps1
.\scripts\package-windows.ps1 -Installer  # Inno Setup 6 が必要
```

詳しい手順は [docs/distribution.md](docs/distribution.md) を参照してください。

---

## 🛠 開発

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
```

- 単体・結合テスト同梱（dB 変換 / ピッチ比 / チェーン / バイパス / リミッター天井 / 設定 / プリセット）
- CI（fmt / clippy / test / build）と Release（各 OS バイナリ / Windows インストーラー）は GitHub Actions に設定済み
- コントリビュート方法は [CONTRIBUTING.md](CONTRIBUTING.md)

---

## 🛡 セーフティ

自分の声の補正・配信品質向上が目的です。次の用途は禁止します:

- 他人へのなりすまし／詐欺・迷惑行為／本人確認の突破
- 無断録音・盗聴／特定人物の声の無断再現

詳細は [docs/safety.md](docs/safety.md)。外部通信は行いません（[SECURITY.md](SECURITY.md)）。

---

## 🗺 ロードマップ

- [x] Phase 1: CLI MVP（入出力・ゲート・EQ・コンプ・リミッター・設定・プリセット）
- [x] Phase 2: 低音ボイス（ピッチ・フォルマント・録音テスト・メーター）
- [x] Phase 3: GUI ダッシュボード（エラーダイアログ・ユーザープリセット保存）
- [x] Phase 4: デバイス互換・安定性（SampleFormat 対応・リサンプリング・バッファ設定・パラメータ差分更新・クロスフェード）
- [x] Phase 4.5: 配信連携ガイド（OBS / Discord / macOS / Linux / ASIO）
- [ ] Phase 5: プラグイン版（CLAP / VST3）
- [ ] Phase 6: AI 拡張（ONNX Runtime・自声補正モデル）

---

## 📄 ライセンス

`MIT OR Apache-2.0`。詳細は [LICENSE](LICENSE)。
