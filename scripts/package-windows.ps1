param(
    [string]$Target = "x86_64-pc-windows-msvc",
    [switch]$SkipBuild,
    [switch]$Installer
)

$ErrorActionPreference = "Stop"

$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$Dist = Join-Path $Root "dist"
$PackageDir = Join-Path $Dist "KuruVoice"

Push-Location $Root
try {
    $versionLine = Select-String -Path "Cargo.toml" -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1
    if (-not $versionLine) {
        throw "Cargo.toml version was not found."
    }
    $Version = $versionLine.Matches[0].Groups[1].Value
    $ReleaseName = "KuruVoice-v$Version-windows-x64"

    if (-not $SkipBuild) {
        cargo build --release --target $Target
    }

    $ExePath = Join-Path $Root "target\$Target\release\kuruvoice.exe"
    if (-not (Test-Path $ExePath)) {
        throw "Built exe was not found: $ExePath"
    }

    New-Item -ItemType Directory -Force $Dist | Out-Null
    if (Test-Path $PackageDir) {
        $resolvedPackageDir = (Resolve-Path $PackageDir).Path
        $resolvedDist = (Resolve-Path $Dist).Path
        if (-not $resolvedPackageDir.StartsWith($resolvedDist, [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "Safety check failed: $resolvedPackageDir"
        }
        Remove-Item -LiteralPath $PackageDir -Recurse -Force
    }

    New-Item -ItemType Directory -Force $PackageDir | Out-Null
    Copy-Item $ExePath $PackageDir
    Copy-Item "README.md" $PackageDir
    Copy-Item "LICENSE" $PackageDir -ErrorAction SilentlyContinue
    Copy-Item "config.example.toml" $PackageDir -ErrorAction SilentlyContinue
    if (Test-Path "docs") {
        Copy-Item "docs" (Join-Path $PackageDir "docs") -Recurse
    }

    $ZipPath = Join-Path $Dist "$ReleaseName.zip"
    if (Test-Path $ZipPath) {
        Remove-Item -LiteralPath $ZipPath -Force
    }
    Compress-Archive -Path $PackageDir -DestinationPath $ZipPath -Force
    $ZipHash = (Get-FileHash $ZipPath -Algorithm SHA256).Hash.ToLowerInvariant()
    "$ZipHash  $(Split-Path $ZipPath -Leaf)" | Out-File "$ZipPath.sha256" -Encoding ascii
    Write-Host "Created: $ZipPath"
    Write-Host "Created: $ZipPath.sha256"

    if ($Installer) {
        $isccCandidates = @(
            (Get-Command "ISCC.exe" -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source -First 1),
            (Join-Path ${env:ProgramFiles(x86)} "Inno Setup 6\ISCC.exe"),
            (Join-Path $env:ProgramFiles "Inno Setup 6\ISCC.exe"),
            (Join-Path $env:LOCALAPPDATA "Programs\Inno Setup 6\ISCC.exe")
        ) | Where-Object { $_ -and (Test-Path $_) }
        $iscc = $isccCandidates | Select-Object -First 1
        if (-not (Test-Path $iscc)) {
            throw "Inno Setup 6 was not found. Install it from https://jrsoftware.org/isinfo.php."
        }

        $env:KURUVOICE_VERSION = $Version
        & $iscc "installer\kuruvoice.iss"

        $installerPath = Join-Path $Dist "$ReleaseName.exe"
        $builtInstaller = Get-ChildItem $Dist -Filter "KuruVoiceSetup-v$Version-windows-x64.exe" | Select-Object -First 1
        if ($builtInstaller) {
            $InstallerHash = (Get-FileHash $builtInstaller.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
            "$InstallerHash  $($builtInstaller.Name)" | Out-File "$($builtInstaller.FullName).sha256" -Encoding ascii
            Write-Host "Created: $($builtInstaller.FullName)"
            Write-Host "Created: $($builtInstaller.FullName).sha256"
        } else {
            throw "Installer output was not found: $installerPath"
        }
    }
} finally {
    Pop-Location
}
