# GT PSP Mod Loader — Setup Script
# =================================
# Installs Python dependencies, verifies the build toolchain,
# and prepares the mod loader environment.

param(
    [switch]$InstallPip
)

$ErrorActionPreference = "Continue"
$ModLoaderDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Resolve-Path "$ModLoaderDir\.."
$AdhocToolchain = "$RepoRoot\workflow\adhoc-toolchain\adhoc.exe"

Write-Host "=== GT PSP Mod Loader Setup ===" -ForegroundColor Cyan
Write-Host ""

# ─── Check Python ─────────────────────────────────────────────────────────
Write-Host "[1/4] Checking Python..." -ForegroundColor Yellow
try {
    $pyVersion = python --version 2>&1
    Write-Host "  OK: $pyVersion" -ForegroundColor Green
} catch {
    Write-Host "  FAILED: Python not found. Install Python 3.8+ from python.org" -ForegroundColor Red
    exit 1
}

# ─── Install Python deps ──────────────────────────────────────────────────
Write-Host "[2/4] Installing Python dependencies..." -ForegroundColor Yellow
$requirements = "$ModLoaderDir\cli\requirements.txt"
if (Test-Path $requirements) {
    $result = pip install -r $requirements 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host "  OK" -ForegroundColor Green
    } else {
        Write-Host "  WARNING: pip install had issues: $result" -ForegroundColor Yellow
    }
} else {
    Write-Host "  SKIP: no requirements.txt" -ForegroundColor Yellow
}

# ─── Check GTAdhocToolchain ───────────────────────────────────────────────
Write-Host "[3/4] Checking GTAdhocToolchain..." -ForegroundColor Yellow
if (Test-Path $AdhocToolchain) {
    Write-Host "  OK: $AdhocToolchain" -ForegroundColor Green
} else {
    Write-Host "  WARNING: adhoc.exe not found at $AdhocToolchain" -ForegroundColor Yellow
    Write-Host "  Download from: https://github.com/Nenkai/GTAdhocToolchain/releases" -ForegroundColor Yellow
}

# ─── Check PPSSPP ─────────────────────────────────────────────────────────
Write-Host "[4/4] Checking PPSSPP..." -ForegroundColor Yellow
$ppssppCandidates = @(
    "$env:USERPROFILE\Documents\PPSSPP",
    "$env:USERPROFILE\.config\ppsspp",
    "C:\Users\$env:USERNAME\Documents\PPSSPP"
)
$found = $false
foreach ($dir in $ppssppCandidates) {
    if (Test-Path "$dir\PSP") {
        Write-Host "  Found: $dir" -ForegroundColor Green
        Write-Host "  UMD replacement: $dir\PSP\UMD0\PSP_GAME\USRDIR\GT.VOL\"
        $found = $true
        break
    }
}
if (-not $found) {
    Write-Host "  WARNING: PPSSPP memstick not found. Set PPSSPP_MEMSTICK env var." -ForegroundColor Yellow
}

Write-Host ""
Write-Host "=== Setup complete ===" -ForegroundColor Cyan
Write-Host ""
Write-Host "Quick start:"
Write-Host "  1. Build modded core:   .\mod_loader\build_modded_core.ps1"
Write-Host "  2. Create a mod:        python mod_loader\cli\gtpsp_mod.py init my_mod"
Write-Host "  3. Build it:            python mod_loader\cli\gtpsp_mod.py build my_mod\"
Write-Host "  4. Deploy to PPSSPP:    python mod_loader\cli\gtpsp_mod.py deploy my_mod\"
