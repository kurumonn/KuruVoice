# KuruVoice 配布手順

KuruVoice は GitHub Releases に zip と Windows インストーラーを置く形で配布します。
タグを push すると GitHub Actions がビルドし、Release Assets を自動作成します。

## 配布物

Windows:

- `KuruVoice-vX.Y.Z-windows-x64.zip`
- `KuruVoiceSetup-vX.Y.Z-windows-x64.exe`
- 各ファイルの `.sha256`

Linux / macOS:

- `KuruVoice-vX.Y.Z-linux-x64.tar.gz`
- `KuruVoice-vX.Y.Z-macos-x64.tar.gz`
- `KuruVoice-vX.Y.Z-macos-arm64.tar.gz`
- 各ファイルの `.sha256`

zip / tar.gz には実行ファイル、`README.md`、`LICENSE`、`config.example.toml`、`docs/` を同梱します。

## ローカルで Windows 配布物を作る

zip のみ:

```powershell
.\scripts\package-windows.ps1
```

Inno Setup 6 をインストール済みなら、インストーラーも作成できます。

```powershell
winget install --id JRSoftware.InnoSetup -e
.\scripts\package-windows.ps1 -Installer
```

生成物は `dist/` に出力されます。

コード署名証明書がある場合は、署名済み配布物も作成できます。

```powershell
$env:KURUVOICE_SIGNING_CERT = "C:\path\to\kuruvoice-signing.pfx"
$env:KURUVOICE_SIGNING_PASSWORD = "pfx-password"
.\scripts\package-windows.ps1 -Installer -Sign
```

## GitHub Release で配布する

1. `Cargo.toml` の `version` とリリース内容を確認する
2. 必要なら `CHANGELOG.md` を更新する
3. タグを作成して push する

```bash
git tag v0.1.0
git push origin v0.1.0
```

Actions の `Release` workflow が成功すると、該当タグの GitHub Release に配布物がアップロードされます。

## Windows 配布時の注意

未署名の exe / installer は Windows SmartScreen で警告されることがあります。初期配布では README や Release note に「未署名のため警告が出る可能性がある」ことを明記し、`.sha256` を併記してください。

一般ユーザー向けの次段階はコード署名です。その後、必要に応じて winget / Scoop への登録を検討します。

コード署名の設定は [docs/code_signing.md](code_signing.md) を参照してください。
