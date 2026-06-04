# KuruVoice 仮想オーディオデバイス内蔵 ― 設計計画

> 目的: KuruVoice **単体**で、加工後の声を「**KuruVoice Virtual Mic**」という入力デバイスとして
> OS に登録し、OBS / Discord / VRChat / Zoom などが**追加ソフト無し**でそれを選べるようにする。
> （現状は VB-CABLE / VoiceMeeter / BlackHole 等の外部仮想ケーブルが必要。）

このドキュメントは実装前の**設計・計画**です。最終的な実装方針（特に Windows）は
[8. 意思決定ポイント](#8-意思決定ポイント) を承認後に着手します。

---

## 0. 用語

| 用語 | 意味 |
|---|---|
| 仮想シンク (virtual sink / render) | アプリが「再生先」として書き込める仮想の出力デバイス |
| 仮想ソース (virtual source / mic) | 他アプリが「マイク」として選べる仮想の入力デバイス |
| ループバック | シンクに書いた音をそのままソースから出す内部結線 |

KuruVoice が欲しいのは **仮想ソース（仮想マイク）**。本体は加工音を仮想シンクへ書き、
ドライバ/プラグインがそれをソースへループバックする、という構成になる。

---

## 1. 最重要の技術的制約（なぜ単純な Rust 1 バイナリでは無理か）

**「他アプリがマイクとして選べるデバイス」は、OS のオーディオサブシステムにデバイスを
登録しなければならない。** これは通常のユーザー空間アプリの権限を超えるため、OS ごとに
実現方式と難易度が大きく異なる。

| OS | 方式 | カーネル | 署名/配布の壁 | 難易度 |
|---|---|---|---|---|
| **Linux** (PipeWire/PulseAudio) | 実行時 API で仮想ソース生成 | 不要 | 不要 | ★☆☆ 容易 |
| **macOS** | AudioServerPlugin (HAL plug-in) | **不要**(ユーザー空間) | 配布時 notarization | ★★☆ 中 |
| **Windows** | AVStream/PortCls 仮想オーディオ**ドライバ** | **必要** | **MS Attestation 署名 + EV 証明書必須** | ★★★ 最難 |

補足:
- Windows 11 には **仮想カメラ**の摩擦の少ない登録 API（`MFCreateVirtualCamera`）があるが、
  **仮想マイクに相当する第一者 API は存在しない**。よって Windows は実質ドライバ一択。
- Rust だけでドライバは書けない（WDK = C/C++）。Windows ドライバは別コンポーネントになる。

> 結論: 「**完全に単体（追加インストール物ゼロ）**」は Linux では可能、macOS では
> ユーザー空間プラグインの同梱で可能、**Windows では署名済みドライバの同梱が必須**で、
> 「単一製品/単一インストーラ」にはできても「単一実行ファイル」にはできない。

---

## 2. 共通アーキテクチャ（出力先の抽象化）

OS 差を吸収するため、DSP の出力先を抽象化する。既存 `src/audio/` を拡張:

```text
DSP スレッド ──(出力リングバッファ)──▶  OutputSink (trait)
                                          ├─ DeviceSink     : 既存の cpal 実デバイス出力
                                          └─ VirtualSink    : 仮想マイクへ送る (OS別バックエンド)
                                                              ├─ linux:  PipeWire/PulseAudio
                                                              ├─ macos:  HAL plug-in へ共有メモリ
                                                              └─ windows: 仮想ドライバの render へ WASAPI
```

```rust
// src/audio/sink/mod.rs （新規・設計案）
pub trait OutputSink: Send {
    fn name(&self) -> String;
    /// モノラル(または2ch)ブロックを出力先へ書き込む。
    fn write(&mut self, block: &[f32]);
    /// 仮想デバイスの生成/破棄を伴う場合の準備・後始末。
    fn start(&mut self) -> anyhow::Result<()>;
    fn stop(&mut self);
}
```

GUI の「出力」セレクタに **「KuruVoice 仮想マイク（内蔵）」** を実デバイスと並べて追加。
選ぶと `Engine` が `DeviceSink` の代わりに `VirtualSink` を使う。これ以外の DSP 経路は不変。

---

## 3. OS 別バックエンド詳細

### 3.1 Linux（最優先 ― 短期で確実に価値が出る）

- **ドライバ不要・署名不要・再起動不要。** 実行時に仮想ソースを作れる。
- 実装案（PipeWire 環境）:
  1. null sink「KuruVoice Sink」を作成
  2. その monitor を remap-source で「**KuruVoice Mic**」として公開
  3. DSP 出力を Sink へ書き込む（PipeWire stream）
- PulseAudio フォールバック: `module-null-sink` + `module-remap-source`。
- 実装手段: `pipewire` crate（libpipewire バインディング）優先、無ければ `pactl`/`pw-cli` を起動。
- 後始末: モジュール unload / stream 破棄で自動消滅。
- 依存が無ければ従来の実デバイス出力にフォールバック。

### 3.2 macOS（中期）

- **AudioServerPlugin（HAL plug-in）= ユーザー空間。カーネル不要。** BlackHole と同方式。
- 配置: `/Library/Audio/Plug-Ins/HAL/KuruVoice.driver`（要管理者）→ `coreaudiod` 再読込。
- 本体 ↔ プラグイン間は **共有メモリ・リングバッファ**で加工音を受け渡す。
- 配布: アプリと .driver の **notarization** が必要。
- **ライセンス注意**: BlackHole は GPLv3。そのまま同梱すると本体まで GPL 化の懸念があるため、
  - HAL プラグインは**自前実装**（Apple サンプル `NullAudio.c` ベース＝寛容ライセンス）にするか、
  - MIT 派生の最小プラグインを用意する。

### 3.3 Windows（長期 ― 本丸、コストと難易度が高い）

- 採用方式: **AVStream/PortCls 仮想オーディオドライバ**（Microsoft **SYSVAD** サンプル派生）。
  仮想キャプチャ（マイク）と仮想レンダー（シンク）のペアを 1 ドライバで提供する。
  - APO（Audio Processing Object）は既存デバイスのパイプライン挿入用で、**新規デバイスの作成には不向き** → 不採用。
- 必須事項:
  - **WDK でドライバ開発（C/C++）。** Rust 本体とは別言語・別ビルド。
  - **INF パッケージ** + 署名: Windows 10/11 では Microsoft の
    **Attestation Signing（パートナーセンター登録 + EV コード署名証明書）**が必須。
  - **インストーラ（管理者権限）**: `pnputil`/`devcon` でドライバ導入、MSI/WiX でパッケージ化、
    アンインストールでドライバ除去。
  - **安定性**: カーネルモードのためバグ = **BSOD**。NF-004（安定性）の延長として厳重テスト必須。
- データ受け渡し: 本体は「**KuruVoice Sink**」へ通常の WASAPI で再生するだけ。
  ドライバ内部で Sink→Mic にループバックする。＝本体コードは普通の出力デバイス書き込みで済む。
- 留意: ドライバ = 攻撃面の増加、年次の証明書コスト、OS 更新での破損リスク、別コンポーネント保守。

---

## 4. Phase 0 ― ドライバを書かずに「ほぼ単体」にする現実解（即着手可能）

Windows で本格ドライバを作るまでの間、**体験を単体に近づける**現実解:

- 起動時に **VB-CABLE / VoiceMeeter の有無を自動検出**。
  - 有り: 出力を自動でその仮想ケーブルへルーティングし、GUI に
    「受け側アプリのマイクで `CABLE Output` を選んでください」と明示・コピー可能表示。
  - 無し: ワンクリックで導入ページを開く案内（自動DLはしない＝[安全方針](#9-完成定義受け入れ基準)）。
- これは**完全単体ではない**が、ユーザーの手間をほぼゼロにできる。`OutputSink` 抽象の上に
  `DetectedCableSink` として実装でき、後続フェーズと無駄にならない。

---

## 5. 段階ロードマップ（推奨実装順）

| Phase | 内容 | 目安 | 単体度 | 状態 |
|---|---|---|---|---|
| **0** | Windows 仮想ケーブル自動検出/誘導 | 数日 | 準単体 | ✅ 実装済み (`src/audio/virtual_cable.rs`) |
| **1** | **Linux** ネイティブ仮想マイク（PipeWire/PulseAudio） | 1–2 週 | 完全単体 | ✅ 実装済み (`src/audio/virtual_mic.rs`、Linux 実機検証は要) |
| **2** | **macOS** HAL プラグイン同梱 + notarization | 数週 | 完全単体 | 未着手 |
| **3** | **Windows** 専用ドライバ + 署名 + インストーラ | 数ヶ月 + 証明書手配 | 完全単体 | **見送り**（EV 証明書が障壁。Phase 0 の仮想ケーブル連携で代替） |

> 方針決定（2026-06-04）: **EV 証明書の壁により Phase 3（Windows 自前ドライバ）は見送り**。
> Windows は Phase 0（既存ケーブル自動連携）を継続利用し、ネイティブ仮想マイクは Linux のみ提供する。

各フェーズは独立して価値が出る順序。まず抽象化（0）を入れ、最も簡単な Linux（1）で
「仮想マイク内蔵」を成立させ、知見を得てから macOS / Windows へ広げる。

---

## 6. リポジトリ構成案

```text
src/audio/
  sink/
    mod.rs            # OutputSink trait + ファクトリ
    device_sink.rs    # 既存 cpal 出力をラップ
    detected_cable.rs # Phase 0: VB-CABLE 等検出ルーティング
    virtual/
      mod.rs
      linux.rs        # PipeWire/PulseAudio 仮想ソース
      macos.rs        # HAL プラグインへの共有メモリ書き込み
      windows.rs      # 仮想ドライバ render への WASAPI 書き込み
drivers/
  windows/            # WDK プロジェクト (C/C++, INF) ※別ビルド
  macos/              # HAL AudioServerPlugin (C, 自前)
installer/
  windows/            # WiX/MSI（ドライバ導入・除去）
  macos/              # pkg（.driver 配置・notarize）
docs/virtual_audio_design.md  # 本書
```

GUI 変更点（`src/gui/dashboard.rs`）:
- 出力セレクタに「KuruVoice 仮想マイク（内蔵）」を追加。
- 仮想デバイス未インストール時はインストールボタン/状態表示を出す。

---

## 7. オーディオ契約（本体 ↔ 仮想デバイス間）

- フォーマット固定: **f32 / 48000Hz**、チャンネルは mono を 1ch、必要に応じ 2ch 複製。
- 受け渡し:
  - Linux: PipeWire stream（OS が同期管理）。
  - macOS: 共有メモリ・リングバッファ（本体が producer、プラグインが consumer）。
  - Windows: 本体→ドライバ render への WASAPI（OS が同期）、ドライバ内で mic へループ。
- 同期/ドロップ: アンダーランは無音補間、オーバーランは古いサンプル破棄（既存方針を踏襲）。
- レイテンシ予算: 既存 DSP（数 ms）＋仮想経路（OS 依存）。目標は往復 < 30ms。

---

## 8. 意思決定ポイント

実装に進む前に確認したい分岐:

1. **Windows の方針**
   - (a) 本格的に **自前ドライバを開発・署名**する（完全単体・高コスト/長期）
   - (b) 当面は **Phase 0（既存ケーブル自動連携）**で妥協し、Linux/macOS を先に完全単体化
2. **対応 OS の優先度**（プロジェクト既定は Windows 優先だが、技術的容易さは Linux > macOS > Windows）
3. **署名/配布コストの許容**（EV 証明書 年 $200–400 + パートナーセンター登録、macOS Apple Developer 年 $99）
4. **ライセンス**（macOS で BlackHole 由来コードを避け自前実装する方針で良いか）

---

## 9. 完成定義（受け入れ基準）

- 各 OS で、他アプリの**入力デバイス一覧に「KuruVoice Virtual Mic」が表示**される。
- それを選ぶと、KuruVoice で加工した声がそのまま流れる。
- KuruVoice 側でバイパス/プリセット変更が即反映される。
- アンインストール（またはアプリ終了）で仮想デバイスが残らない。
- 外部通信を行わない（NF-008）。ドライバ/プラグインは攻撃面を最小化。
- 過大音量はリミッター最終段で防ぐ（NF-005）＝仮想マイクへ出す前に必ず通す。

---

## 10. まとめ

- **抽象化（OutputSink）→ Linux → macOS → Windows** の順で進めるのが最短かつ堅実。
- 「アプリ単体で完結」は **Linux/macOS では真に実現可能**、**Windows は署名済みドライバ同梱が条件**。
- すぐ着手できる価値は **Phase 0（出力抽象化＋既存ケーブル自動連携）** と **Phase 1（Linux）**。
