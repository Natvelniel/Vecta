param(
    [string]$Alias = "vecta",
    [string]$Password = "",
    [switch]$Force
)

$ErrorActionPreference = "Stop"

function Find-Keytool {
    $cmd = Get-Command keytool -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }

    $candidates = @()
    if ($env:JAVA_HOME) {
        $candidates += Join-Path $env:JAVA_HOME "bin\keytool.exe"
    }
    $searchRoots = @(
        "C:\Program Files\Java",
        "C:\Program Files\Eclipse Adoptium",
        "C:\Program Files\Microsoft",
        "C:\Program Files\Android\Android Studio\jbr",
        "C:\Program Files (x86)\Java"
    )
    foreach ($root in $searchRoots) {
        if (Test-Path $root) {
            $found = Get-ChildItem -Path $root -Recurse -Filter "keytool.exe" -ErrorAction SilentlyContinue | Select-Object -First 1
            if ($found) { return $found.FullName }
        }
    }
    throw "keytool.exe tidak ditemukan. Install JDK 17+ (atau Eclipse Temurin / Android Studio) lalu jalankan ulang script ini."
}

$keytool = Find-Keytool
$outDir = Join-Path $HOME ".vecta-signing"
New-Item -ItemType Directory -Force -Path $outDir | Out-Null

$ksPath = Join-Path $outDir "vecta-release.jks"
$b64Path = Join-Path $outDir "ANDROID_KEYSTORE_BASE64.txt"

if ((Test-Path $ksPath) -and -not $Force) {
    Write-Host "Keystore sudah ada di: $ksPath" -ForegroundColor Yellow
    Write-Host "Gunakan -Force hanya jika ingin membuat keystore baru (PERINGATAN: APK yang sudah terpasang harus uninstall sekali jika key berubah)." -ForegroundColor Yellow
    exit 0
}

if ([string]::IsNullOrWhiteSpace($Password)) {
    $bytes = New-Object byte[] 24
    [System.Security.Cryptography.RandomNumberGenerator]::Create().GetBytes($bytes)
    $Password = [Convert]::ToBase64String($bytes) -replace '[+/=]', ''
}

if (Test-Path $ksPath) {
    Remove-Item $ksPath -Force
}

& $keytool -genkeypair -noprompt `
    -keystore $ksPath `
    -storetype PKCS12 `
    -alias $Alias `
    -keyalg RSA `
    -keysize 2048 `
    -validity 10000 `
    -storepass $Password `
    -keypass $Password `
    -dname "CN=Vecta, OU=Release, O=Natvelniel, L=Indonesia, S=ID, C=ID"

if ($LASTEXITCODE -ne 0) {
    throw "keytool gagal membuat keystore."
}

$ksBytes = [System.IO.File]::ReadAllBytes($ksPath)
$b64 = [Convert]::ToBase64String($ksBytes)
[System.IO.File]::WriteAllText($b64Path, $b64)

Write-Host ""
Write-Host "=== ANDROID RELEASE KEYSTORE SIAP ===" -ForegroundColor Green
Write-Host "Keystore file : $ksPath"
Write-Host "Base64 file   : $b64Path"
Write-Host ""
& $keytool -list -v -keystore $ksPath -storepass $Password | Select-String -Pattern "Alias name:|SHA256:"
Write-Host ""
Write-Host "Tambahkan 4 GitHub Repository Secrets di: Settings -> Secrets and variables -> Actions -> New repository secret" -ForegroundColor Cyan
Write-Host "1. ANDROID_KEYSTORE_BASE64   : (salin seluruh isi file $b64Path)"
Write-Host "2. ANDROID_KEYSTORE_PASSWORD : $Password"
Write-Host "3. ANDROID_KEY_ALIAS         : $Alias"
Write-Host "4. ANDROID_KEY_PASSWORD      : $Password"
Write-Host ""
Write-Host "Penting: Simpan file $ksPath dan password ini dengan aman. Selama keystore ini dipakai,"
Write-Host "rilisan Vecta v0.5.2, v0.5.3, dst. bisa langsung Update di Android tanpa 'package berbeda'."
