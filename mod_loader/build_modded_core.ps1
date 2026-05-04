# GT PSP — Build Modded Core Scripts
# ====================================
# Compiles the patched Application.ad and packed_main_loop into .adc,
# then deploys them to test_output and optionally to PPSSPP.
#
# Usage:
#   .\mod_loader\build_modded_core.ps1                        # Build only
#   .\mod_loader\build_modded_core.ps1 -DeployToPPSSPP        # Build + deploy to PPSSPP
#   .\mod_loader\build_modded_core.ps1 -DeployToGTVol         # Build + deploy to GT.VOL
#

param(
    [switch]$DeployToPPSSPP,
    [switch]$DeployToGTVol
)

$RepoRoot = "D:\GTPSP-decompile"
$AdhocToolchain = "$RepoRoot\workflow\adhoc-toolchain\adhoc.exe"
$ModLoaderDir = "$RepoRoot\mod_loader"
$OutputDir = "$RepoRoot\test_output\modded_core"
$ErrorActionPreference = "Stop"

Write-Host "=== GT PSP Modded Core Builder ===" -ForegroundColor Cyan
$totalSuccess = $true

# ─── Step 1: Build patched Application.ad ─────────────────────────────────
Write-Host "[1/3] Building patched Application.ad..." -ForegroundColor Yellow
$appAd = "$ModLoaderDir\core\Application_patched.ad"
$appAdc = "$ModLoaderDir\core\Application_patched.adc"
try {
    & $AdhocToolchain build -i $appAd -o $appAdc -v 12 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "adhoc.exe failed" }
    Write-Host "  OK: Application_patched.adc" -ForegroundColor Green
} catch {
    Write-Host "  FAILED: Application_patched.ad" -ForegroundColor Red
    $totalSuccess = $false
}

# ─── Step 2: Build patched packed_main_loop ───────────────────────────────
Write-Host "[2/3] Building patched packed_main_loop..." -ForegroundColor Yellow
$mainLoopAd = "$ModLoaderDir\core\main_loop_patched.ad"
$mainLoopAdc = "$ModLoaderDir\core\packed_main_loop.adc"
try {
    & $AdhocToolchain build -i $mainLoopAd -o $mainLoopAdc -v 12 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "adhoc.exe failed" }
    Write-Host "  OK: packed_main_loop.adc" -ForegroundColor Green
} catch {
    Write-Host "  FAILED: packed_main_loop.ad" -ForegroundColor Red
    $totalSuccess = $false
}

if (-not $totalSuccess) {
    Write-Host "`nBuild failed. Aborting." -ForegroundColor Red
    exit 1
}

# ─── Step 3: Deploy to test output ────────────────────────────────────────
Write-Host "[3/3] Deploying to test_output..." -ForegroundColor Yellow
New-Item -ItemType Directory -Path "$OutputDir\scripts\gt5m" -Force | Out-Null
Copy-Item $appAdc "$OutputDir\scripts\gt5m\Application.adc" -Force
Copy-Item $mainLoopAdc "$OutputDir\scripts\gt5m\packed_main_loop.adc" -Force

$appSize = (Get-Item "$OutputDir\scripts\gt5m\Application.adc").Length
$pmlSize = (Get-Item "$OutputDir\scripts\gt5m\packed_main_loop.adc").Length
Write-Host "  Output: $OutputDir" -ForegroundColor Green
Write-Host "    scripts/gt5m/Application.adc ($appSize bytes)"
Write-Host "    scripts/gt5m/packed_main_loop.adc ($pmlSize bytes)"

# ─── Deploy to PPSSPP ─────────────────────────────────────────────────────
if ($DeployToPPSSPP) {
    $candidates = @(
        "$env:USERPROFILE\Documents\PPSSPP",
        "C:\Users\$env:USERNAME\Documents\PPSSPP"
    )
    $found = $false
    foreach ($dir in $candidates) {
        $umdDir = "$dir\PSP\UMD0\PSP_GAME\USRDIR\GT.VOL\scripts\gt5m"
        if (Test-Path "$dir\PSP") {
            New-Item -ItemType Directory -Path $umdDir -Force | Out-Null
            Copy-Item "$OutputDir\scripts\gt5m\Application.adc" "$umdDir\" -Force
            Copy-Item "$OutputDir\scripts\gt5m\packed_main_loop.adc" "$umdDir\" -Force
            Write-Host "  Deployed to PPSSPP: $umdDir" -ForegroundColor Green
            $found = $true
            break
        }
    }
    if (-not $found) {
        Write-Host "  PPSSPP not found. Copy manually from: $OutputDir" -ForegroundColor Yellow
    }
}

if ($DeployToGTVol) {
    $gtvolDir = "$RepoRoot\files\decompiled\Gran Turismo\PSP_GAME\USRDIR\GT.VOL\scripts\gt5m"
    if (Test-Path $gtvolDir) {
        Copy-Item "$OutputDir\scripts\gt5m\Application.adc" "$gtvolDir\" -Force
        Copy-Item "$OutputDir\scripts\gt5m\packed_main_loop.adc" "$gtvolDir\" -Force
        Write-Host "  Deployed to GT.VOL: $gtvolDir" -ForegroundColor Green
    }
}

Write-Host "`n=== Build complete ===" -ForegroundColor Cyan
Write-Host "Next: Deploy test_output\modded_core to PPSSPP memstick and launch."
