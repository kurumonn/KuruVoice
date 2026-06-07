# macOS セットアップガイド（T-011）

KuruVoice を macOS で使うための手順です。  
macOS は CoreAudio を使うため、仮想オーディオデバイスは **BlackHole** または **Loopback** を使います。

---

## 前提

| 項目 | 要件 |
|------|------|
| macOS | 12 Monterey 以上（Apple Silicon / Intel 両対応） |
| Rust | 1.75 以上 ([rustup.rs](https://rustup.rs/)) |
| Xcode CLI Tools | `xcode-select --install` |

---

## 1. Rust と依存ツールを入れる

```bash
# rustup が未インストールの場合
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Xcode CLI ツール（すでに入っている場合は不要）
xcode-select --install
```

---

## 2. KuruVoice をビルドする

```bash
git clone https://github.com/kurumonn/KuruVoice.git
cd KuruVoice
cargo build --release
```

> Apple Silicon Mac（M1/M2/M3）では `aarch64-apple-darwin` ターゲットが自動的に使われます。

---

## 3. 仮想オーディオデバイスを導入する

KuruVoice の加工音を OBS や Discord に送るには仮想オーディオデバイスが必要です。

### 無料: BlackHole

1. [https://existential.audio/blackhole/](https://existential.audio/blackhole/) からインストール
2. **BlackHole 2ch** を選択（16ch は高帯域用途向け）
3. インストール後、`システム設定 → サウンド` で "BlackHole 2ch" が表示されることを確認

### 有料: Loopback（GUI で柔軟に設定したい場合）

[Rogue Amoeba Loopback](https://rogueamoeba.com/loopback/) は GUI で仮想デバイスを自在に作成でき、
KuruVoice の出力を複数アプリに同時ルーティングできます。

---

## 4. KuruVoice を起動して仮想デバイスへ出力する

```bash
./target/release/kuruvoice
```

1. 左ペインの **入力デバイス** でマイクを選択
2. **出力デバイス** で "BlackHole 2ch"（または Loopback で作った仮想デバイス）を選択
3. 上部の **▶ 開始** を押す

---

## 5. OBS / Discord で仮想デバイスを選ぶ

| アプリ | 設定箇所 | 選ぶデバイス名 |
|--------|---------|--------------|
| OBS | ソース追加 → 音声入力キャプチャ | BlackHole 2ch |
| Discord | 設定 → 音声・ビデオ → 入力デバイス | BlackHole 2ch |
| Zoom | 設定 → オーディオ → マイク | BlackHole 2ch |

詳細は [obs_integration.md](obs_integration.md) と [discord_integration.md](discord_integration.md) を参照。

---

## macOS Notarization（バイナリを他者に配布する場合）

Apple のセキュリティポリシー上、配布バイナリは公証（notarization）が必要です。

1. Apple Developer アカウント（有料: $99/年）を取得
2. コード署名:
   ```bash
   codesign --deep --force --verify --verbose \
     --sign "Developer ID Application: Your Name (XXXXXXXX)" \
     target/release/kuruvoice
   ```
3. 公証に提出:
   ```bash
   xcrun notarytool submit kuruvoice.zip \
     --apple-id you@example.com --team-id XXXXXXXX --wait
   ```
4. ステープル:
   ```bash
   xcrun stapler staple target/release/kuruvoice
   ```

個人使用・開発目的のみなら公証は不要です（「開発元を確認できません」ダイアログで「開く」を選択）。

---

## トラブルシューティング

| 症状 | 対処法 |
|------|-------|
| マイクのアクセス許可が出ない | システム設定 → プライバシーとセキュリティ → マイク → kuruvoice を許可 |
| BlackHole が選択肢に出ない | ログアウト/再ログイン後に再確認 |
| 音が出ない / バッファ落ち | `config.toml` の `buffer_size = 512` に増やす |
| Apple Silicon でクラッシュ | `rustup target add aarch64-apple-darwin` して再ビルド |

---

## 関連ドキュメント

- [setup_linux.md](setup_linux.md) — Linux セットアップ
- [asio_setup.md](asio_setup.md) — Windows ASIO 低遅延設定
- [obs_integration.md](obs_integration.md) — OBS 連携手順
