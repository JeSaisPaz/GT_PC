# GT PSP (gt5m) - Full Build Script
# Compiles all .ad source files to .adc bytecode using GTAdhocToolchain

$ErrorActionPreference = "Continue"
$adhoc = "D:\GTPSP-decompile\workflow\adhoc-toolchain\adhoc.exe"
$source = "D:\GTPSP-decompile\source"
$testOutput = "D:\GTPSP-decompile\test_output"

# ---- Step 1: Fix known decompiler artifacts ----
Write-Host "=== Fixing known decompiler artifacts ===" -ForegroundColor Cyan
$fixFile = Join-Path $source "scripts\gt5m\road_sound_autogen.ad"
if (Test-Path $fixFile) {
    $content = Get-Content $fixFile -Raw
    $fixed = $content -replace '(\d+\.\d+)f', '$1' -replace '(?<!\d\.)(\d+)f(?!\d)', '$1.0'
    Set-Content $fixFile $fixed -NoNewline
    Write-Host "  Fixed f suffixes in road_sound_autogen.ad" -ForegroundColor Green
}

# ---- Step 2: Build YAML projects ----
Write-Host "`n=== Building YAML projects ===" -ForegroundColor Cyan
$yamlFiles = Get-ChildItem -Path $source -Filter "*.yaml" -Recurse
foreach ($yaml in $yamlFiles) {
    $outDir = $yaml.DirectoryName
    $baseName = $yaml.BaseName
    $outFile = Join-Path $outDir "$baseName.adc"
    
    Write-Host "  Building $baseName..." -NoNewline
    $result = & $adhoc build -i $yaml.FullName -o $outFile 2>&1
    if ($LASTEXITCODE -eq 0 -and $?) {
        Write-Host " OK" -ForegroundColor Green
    } else {
        Write-Host " FAILED" -ForegroundColor Red
        Write-Host "    $result" -ForegroundColor DarkRed
    }
}

# ---- Step 3: Build standalone .ad files ----
Write-Host "`n=== Building standalone .ad files ===" -ForegroundColor Cyan
$adFiles = Get-ChildItem -Path $source -Filter "*.ad" -Recurse | Where-Object {
    # Skip files that would be compiled as part of a YAML project
    # (We don't have an easy way to know which are standalone vs part-of-project,
    #  so we just try to build individual files that have no yaml companion)
    $dir = $_.DirectoryName
    $hasYaml = (Get-ChildItem $dir -Filter "*.yaml").Count -gt 0
    $isMainLoop = $_.Name -eq "main_loop.ad" -or $_.Name -eq "road_sound_autogen.ad"
    $isGlobalStatus = $_.DirectoryName -like "*global_status*"
    $isShare = $_.DirectoryName -like "*share*"
    
    -not $hasYaml -and -not $isMainLoop -and -not $isGlobalStatus -and -not $isShare
}

# Also build specific standalone files that are NOT included by others
$standaloneScripts = @(
    "$source\scripts\gt5m\Application.ad",
    "$source\scripts\gt5m\bootstrap.ad",
    "$source\scripts\gt5m\bootstrap_phase2.ad",
    "$source\scripts\gt5m\init_sound.ad",
    "$source\scripts\gt5m\shutdown.ad",
    "$source\projects\gt5m\menuinit.ad",
    "$source\products\gt5m\script\MenuClassDefine.ad"
)

foreach ($ad in $standaloneScripts) {
    if (-not (Test-Path $ad)) { continue }
    $outFile = [System.IO.Path]::ChangeExtension($ad, ".adc")
    $name = [System.IO.Path]::GetFileNameWithoutExtension($ad)
    
    Write-Host "  Building $name..." -NoNewline
    $result = & $adhoc build -i $ad -o $outFile -v 12 2>&1
    if ($LASTEXITCODE -eq 0 -and $?) {
        Write-Host " OK" -ForegroundColor Green
    } else {
        Write-Host " FAILED" -ForegroundColor Red
        Write-Host "    $result" -ForegroundColor DarkRed
    }
}

# ---- Step 4: Build util scripts ----
Write-Host "`n=== Building util scripts ===" -ForegroundColor Cyan
$utilScripts = Get-ChildItem -Path "$source\scripts\gt5m\util" -Filter "*.ad"
foreach ($ad in $utilScripts) {
    $outFile = [System.IO.Path]::ChangeExtension($ad.FullName, ".adc")
    $name = $ad.BaseName
    
    Write-Host "  Building $name..." -NoNewline
    $result = & $adhoc build -i $ad.FullName -o $outFile -v 12 2>&1
    if ($LASTEXITCODE -eq 0 -and $?) {
        Write-Host " OK" -ForegroundColor Green
    } else {
        Write-Host " FAILED" -ForegroundColor Red
        Write-Host "    $result" -ForegroundColor DarkRed
    }
}

Write-Host "`n=== Build complete ===" -ForegroundColor Cyan
