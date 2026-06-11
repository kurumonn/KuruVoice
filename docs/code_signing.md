# Windows コード署名

Windows の未署名 exe / installer は SmartScreen で警告されることがあります。警告を減らすには、コード署名証明書で `kuruvoice.exe` と `KuruVoiceSetup-*.exe` に署名します。

## GitHub Actions で署名する

Release workflow は、次の GitHub Secrets が設定されている場合だけ Windows 配布物を署名します。

- `WINDOWS_CERTIFICATE_BASE64`: PFX 証明書を Base64 化した文字列
- `WINDOWS_CERTIFICATE_PASSWORD`: PFX 証明書のパスワード

PowerShell で PFX を Base64 化する例:

```powershell
[Convert]::ToBase64String([IO.File]::ReadAllBytes("kuruvoice-signing.pfx")) |
  Set-Content -Path "kuruvoice-signing.pfx.base64" -Encoding ascii
```

`kuruvoice-signing.pfx.base64` の内容を `WINDOWS_CERTIFICATE_BASE64` に登録します。

Secrets 未設定の場合、Release workflow は署名をスキップして従来通り未署名の配布物を作ります。

## ローカルで署名する

Windows SDK または Visual Studio Build Tools の `signtool.exe` が必要です。

```powershell
$env:KURUVOICE_SIGNING_CERT = "C:\path\to\kuruvoice-signing.pfx"
$env:KURUVOICE_SIGNING_PASSWORD = "pfx-password"

.\scripts\package-windows.ps1 -Installer -Sign
```

既存ファイルだけ署名する場合:

```powershell
.\scripts\sign-windows.ps1 -FilePath .\target\x86_64-pc-windows-msvc\release\kuruvoice.exe
.\scripts\sign-windows.ps1 -FilePath .\dist\KuruVoiceSetup-v0.1.0-windows-x64.exe
```

## 証明書の選び方

初期段階では標準のコード署名証明書でも配布元の確認には有効です。ただし SmartScreen の警告を大きく減らすには実績の蓄積が必要です。即効性を重視する場合は EV コード署名証明書を検討してください。

署名後も Release Assets の `.sha256` は公開し続けます。
