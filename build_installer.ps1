# build_installer.ps1 - Automated build & installer generator script
Write-Host "=========================================" -ForegroundColor Cyan
Write-Host " Building MiniLyrics V2 Release & Setup  " -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Cyan

$ErrorActionPreference = "Stop"
$WorkingDir = Get-Location

Write-Host "`n[1/3] Compiling Release Binary via cargo build --release..." -ForegroundColor Yellow
cargo build --release
if ($LASTEXITCODE -ne 0) {
    Write-Host "Cargo build failed!" -ForegroundColor Red
    exit 1
}

$ExePath = Join-Path $WorkingDir "target\release\minilyricv2.exe"
if (-not (Test-Path $ExePath)) {
    Write-Host "Release binary not found at $ExePath" -ForegroundColor Red
    exit 1
}

Write-Host "Release binary compiled successfully: $ExePath" -ForegroundColor Green

Write-Host "`n[2/3] Checking for Inno Setup Compiler (ISCC.exe)..." -ForegroundColor Yellow
$IsccPaths = @(
    "ISCC.exe",
    "C:\Program Files (x86)\Inno Setup 6\ISCC.exe",
    "C:\Program Files\Inno Setup 6\ISCC.exe",
    "C:\Program Files (x86)\Inno Setup 5\ISCC.exe"
)

$IsccFound = $null
foreach ($path in $IsccPaths) {
    if (Get-Command $path -ErrorAction SilentlyContinue) {
        $IsccFound = $path
        break
    } elseif (Test-Path $path) {
        $IsccFound = $path
        break
    }
}

$DistDir = Join-Path $WorkingDir "dist"
if (-not (Test-Path $DistDir)) {
    New-Item -ItemType Directory -Path $DistDir | Out-Null
}

if ($IsccFound) {
    Write-Host "Found Inno Setup Compiler at: $IsccFound" -ForegroundColor Green
    Write-Host "`n[3/3] Generating Setup Installer using installer.iss..." -ForegroundColor Yellow
    & $IsccFound "installer.iss"
    if ($LASTEXITCODE -eq 0) {
        Write-Host "`nSUCCESS! Installer created successfully in output directory." -ForegroundColor Green
    } else {
        Write-Host "ISCC compilation failed." -ForegroundColor Red
    }
} else {
    Write-Host "ISCC.exe not found on system. Creating dist ZIP archive as fallback..." -ForegroundColor Yellow
    $ZipPath = Join-Path $DistDir "MiniLyricsV2_v0.1.10.zip"
    if (Test-Path $ZipPath) { Remove-Item $ZipPath -Force }
    Compress-Archive -Path $ExePath -DestinationPath $ZipPath
    Write-Host "`nFallback ZIP created at: $ZipPath" -ForegroundColor Green
    Write-Host "To generate installer EXE, install Inno Setup 6 (https://jrsoftware.org/isdl.php)." -ForegroundColor Cyan
}

Write-Host "`nDone!" -ForegroundColor Green
