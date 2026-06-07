# ASIO 対応ビルド手順（T-018）

Windows 向けの超低遅延設定です。ASIO は Steinberg が策定したプロ向け低遅延 API で、
標準の WASAPI より 1〜3 ms 低い遅延を実現できます。

---

## 前提

| 項目 | 要件 |
|------|------|
| OS | Windows 10/11（64 bit） |
| ドライバ | ASIO 対応オーディオインターフェース（例: FOCUSRITE Scarlett, Native Instruments など）または [ASIO4ALL](https://www.asio4all.org/)（汎用ドライバ） |
| ASIO SDK | Steinberg ASIO SDK（[登録不要で無償取得可](https://www.steinberg.net/asiosdk)） |
| Rust | 1.75 以上 |
| MSVC | Visual Studio Build Tools（C++ コンポーネント含む） |

---

## 手順

### 1. ASIO SDK を入手する

1. [https://www.steinberg.net/asiosdk](https://www.steinberg.net/asiosdk) にアクセス
2. ダウンロードした ZIP を任意のパスに展開する（例: `C:\asio_sdk`）
3. 展開後のフォルダ構造確認:
   ```
   C:\asio_sdk\
   ├── common\
   │   ├── asio.h
   │   └── ...
   └── ...
   ```

### 2. 環境変数を設定する

```powershell
$env:CPAL_ASIO_DIR = "C:\asio_sdk"
```

永続化する場合はシステムの環境変数設定（`sysdm.cpl` → 詳細 → 環境変数）で追加してください。

### 3. ASIO feature を有効にしてビルドする

```powershell
cargo build --release --features asio
```

> 初回は cpal が ASIO SDK のヘッダをバインドするため通常よりコンパイルに時間がかかります。

### 4. 起動する

```powershell
.\target\release\kuruvoice.exe
```

左ペインのデバイス選択に "ASIO: <デバイス名>" が表示されれば成功です。

---

## バッファサイズの調整

ASIO では `config.toml` の `[audio] buffer_size` をデバイスがサポートする最小値（例: 64〜128）に設定できます。

```toml
[audio]
buffer_size = 64   # ASIO 推奨: 64〜128
```

WASAPI では 256 未満にすると不安定になることがありますが、ASIO ではデバイス依存で 32〜64 まで下げられます。

---

## ASIO4ALL（専用インターフェースがない場合）

一般的なマザーボード内蔵サウンドカードでも ASIO4ALL を使えば低遅延モードに近づけます。

1. [https://www.asio4all.org/](https://www.asio4all.org/) からインストール
2. 上記手順と同様に `CPAL_ASIO_DIR` を設定してビルド
3. ASIO4ALL のコントロールパネルでバッファサイズを設定（256〜512 が安定します）

---

## トラブルシューティング

| 症状 | 対処法 |
|------|-------|
| `CPAL_ASIO_DIR` not set エラー | 環境変数を再確認。PowerShell セッションを開き直す |
| `asio.h` not found | SDK フォルダの `common/` パスを確認 |
| デバイスが表示されない | ASIO ドライバが正しくインストールされているか確認 |
| 音が途切れる | バッファサイズを大きくする（128→256）、他のオーディオアプリを閉じる |

---

## 関連ドキュメント

- [setup_macos.md](setup_macos.md) — macOS CoreAudio の低遅延設定
- [setup_linux.md](setup_linux.md) — Linux JACK バックエンドの設定
- [docs/design.md](design.md) — エンジンアーキテクチャ
