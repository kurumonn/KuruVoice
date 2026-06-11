param(
    [Parameter(Mandatory = $true)]
    [string[]]$FilePath,

    [string]$CertificatePath = $env:KURUVOICE_SIGNING_CERT,
    [string]$CertificatePassword = $env:KURUVOICE_SIGNING_PASSWORD,
    [string]$TimestampUrl = "http://timestamp.digicert.com",
    [switch]$SkipIfMissing
)

$ErrorActionPreference = "Stop"

function Find-SignTool {
    $fromPath = Get-Command "signtool.exe" -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source -First 1
    if ($fromPath -and (Test-Path $fromPath)) {
        return $fromPath
    }

    $kitRoots = @(
        (Join-Path ${env:ProgramFiles(x86)} "Windows Kits\10\bin"),
        (Join-Path $env:ProgramFiles "Windows Kits\10\bin")
    )

    foreach ($root in $kitRoots) {
        if (-not (Test-Path $root)) {
            continue
        }
        $candidate = Get-ChildItem -Path $root -Filter "signtool.exe" -Recurse -ErrorAction SilentlyContinue |
            Where-Object { $_.FullName -match "\\x64\\signtool\.exe$" } |
            Sort-Object FullName -Descending |
            Select-Object -ExpandProperty FullName -First 1
        if ($candidate) {
            return $candidate
        }
    }

    return $null
}

if (-not $CertificatePath -or -not (Test-Path $CertificatePath)) {
    if ($SkipIfMissing) {
        Write-Host "Signing certificate was not found. Skipping signing."
        exit 0
    }
    throw "Signing certificate was not found. Set KURUVOICE_SIGNING_CERT or pass -CertificatePath."
}

$signTool = Find-SignTool
if (-not $signTool) {
    if ($SkipIfMissing) {
        Write-Host "signtool.exe was not found. Skipping signing."
        exit 0
    }
    throw "signtool.exe was not found. Install Windows SDK or Visual Studio Build Tools."
}

foreach ($path in $FilePath) {
    if (-not (Test-Path $path)) {
        throw "File to sign was not found: $path"
    }

    $args = @(
        "sign",
        "/fd", "SHA256",
        "/td", "SHA256",
        "/tr", $TimestampUrl,
        "/f", $CertificatePath
    )
    if ($CertificatePassword) {
        $args += @("/p", $CertificatePassword)
    }
    $args += $path

    & $signTool @args
    if ($LASTEXITCODE -ne 0) {
        throw "signtool sign failed for $path"
    }

    & $signTool verify /pa /v $path
    if ($LASTEXITCODE -ne 0) {
        throw "signtool verify failed for $path"
    }
}
