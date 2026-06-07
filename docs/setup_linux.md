# Linux セットアップガイド（T-011）

KuruVoice を Linux で使うための手順です。  
PipeWire（推奨）または PulseAudio + 仮想ソースで動作します。

---

## 前提

| 項目 | 要件 |
|------|------|
| ディストリビューション | Ubuntu 22.04+ / Fedora 38+ / Arch Linux など |
| オーディオシステム | PipeWire 0.3+ または PulseAudio |
| Rust | 1.75 以上 |

---

## 1. ビルド依存をインストールする

```bash
# Ubuntu / Debian
sudo apt-get update
sudo apt-get install -y \
    build-essential pkg-config \
    libasound2-dev \
    libpipewire-0.3-dev \
    libclang-dev

# Fedora
sudo dnf install -y \
    gcc pkg-config \
    alsa-lib-devel \
    pipewire-devel \
    clang-devel

# Arch Linux
sudo pacman -S --needed \
    base-devel pkg-config \
    alsa-lib pipewire clang
```

---

## 2. Rust をインストールする

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

---

## 3. KuruVoice をビルドする

```bash
git clone https://github.com/kurumonn/KuruVoice.git
cd KuruVoice
cargo build --release
```

---

## 4. 仮想オーディオソースを作成する

### PipeWire（推奨）

PipeWire 0.3 以降では `pw-loopback` または `pactl` で仮想ソースを作れます。

```bash
# KuruVoice 出力を受け取る仮想シンク（録音ソース付き）を作成
pw-loopback --capture-props='media.class=Audio/Source' \
            --playback-props='media.class=Audio/Sink' \
            --name KuruVoice_Mic &
```

起動後、`pw-top` で "KuruVoice_Mic" が表示されれば成功です。

```bash
# セッション終了後に削除
pkill pw-loopback
```

#### 永続化（systemd ユーザーサービスとして登録）

```bash
mkdir -p ~/.config/systemd/user
cat > ~/.config/systemd/user/kuruvoice-loopback.service << 'EOF'
[Unit]
Description=KuruVoice virtual mic loopback

[Service]
ExecStart=pw-loopback \
  --capture-props="media.class=Audio/Source" \
  --playback-props="media.class=Audio/Sink" \
  --name KuruVoice_Mic
Restart=on-failure

[Install]
WantedBy=default.target
EOF

systemctl --user enable --now kuruvoice-loopback.service
```

---

### PulseAudio（PipeWire を使っていない場合）

```bash
# 仮想シンク（出力先）を作成
pactl load-module module-null-sink \
    sink_name=kuruvoice_sink \
    sink_properties=device.description="KuruVoice_Sink"

# シンクのモニター（他アプリのマイク入力として使う）
# → ソース名: kuruvoice_sink.monitor
pactl list sources short | grep kuruvoice
```

`kuruvoice_sink.monitor` を OBS や Discord のマイク入力に設定してください。

---

## 5. KuruVoice を起動する

```bash
./target/release/kuruvoice
```

1. 入力デバイス: 実マイク
2. 出力デバイス: `KuruVoice_Mic`（PipeWire）または `kuruvoice_sink`（PulseAudio）
3. **▶ 開始** を押す

---

## 6. OBS / Discord で仮想マイクを選ぶ

| アプリ | 設定箇所 | 選ぶデバイス名 |
|--------|---------|--------------|
| OBS | ソース → 音声入力キャプチャ | KuruVoice_Mic |
| Discord | 設定 → 音声・ビデオ → 入力デバイス | KuruVoice_Mic |

---

## JACK バックエンド（プロ向け超低遅延）

```bash
# JACK 依存を追加ビルド（将来の --features jack 対応）
sudo apt-get install -y libjack-jackd2-dev
cargo build --release --features jack   # 現在開発中
```

JACK は PipeWire より設定が複雑ですが 1〜2 ms の超低遅延を実現できます。  
`qjackctl` で Jack サーバを起動してから kuruvoice を起動してください。

---

## トラブルシューティング

| 症状 | 対処法 |
|------|-------|
| `libasound2` not found | `sudo apt-get install libasound2-dev` |
| デバイスが空 | `aplay -l` でデバイス検出を確認、`ALSA_CARD` 環境変数を設定 |
| PipeWire で音が出ない | `systemctl --user restart pipewire pipewire-pulse` |
| バッファ落ち | `config.toml` の `buffer_size` を 512 に増やす |

---

## 関連ドキュメント

- [setup_macos.md](setup_macos.md) — macOS セットアップ
- [asio_setup.md](asio_setup.md) — Windows ASIO 低遅延設定
- [obs_integration.md](obs_integration.md) — OBS 連携手順
