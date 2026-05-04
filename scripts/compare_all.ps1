# GT PSP (gt5m) - Compare Recompiled vs Original .adc
# Uses GTAdhocCompare.py to generate HTML diff reports

$adhocDir = "D:\GTPSP-decompile\workflow\adhoc-toolchain"
$comparePy = Join-Path $adhocDir "GTAdhocCompare.py"
$source = "D:\GTPSP-decompile\source"
$original = "D:\GTPSP-decompile\files\decompiled\Gran Turismo\PSP_GAME\USRDIR\GT.VOL"
$outputDir = "D:\GTPSP-decompile\test_output\comparisons"

# Add adhoc.exe to PATH for compare script
$env:PATH = "$adhocDir;$env:PATH"

# Create output directory
New-Item -ItemType Directory -Path $outputDir -Force | Out-Null

# Define mappings: source .adc -> original .adc
$mappings = @(
    @{ Source = "$source\scripts\gt5m\init_sound.adc"; Original = "$original\scripts\gt5m\init_sound.adc"; Name = "init_sound" },
    @{ Source = "$source\scripts\gt5m\packed_main_loop.adc"; Original = "$original\scripts\gt5m\packed_main_loop.adc"; Name = "packed_main_loop" },
    @{ Source = "$source\scripts\gt5m\shutdown.adc"; Original = "$original\scripts\gt5m\shutdown.adc"; Name = "shutdown" },
    @{ Source = "$source\scripts\gt5m\global_status\packed_global_status.adc"; Original = "$original\scripts\gt5m\global_status\packed_global_status.adc"; Name = "packed_global_status" },
    
    @{ Source = "$source\projects\gt5m\arcade\arcade.adc"; Original = "$original\projects\gt5m\arcade\arcade.adc"; Name = "arcade" },
    @{ Source = "$source\projects\gt5m\boot\boot.adc"; Original = "$original\projects\gt5m\boot\boot.adc"; Name = "boot" },
    @{ Source = "$source\projects\gt5m\cursor\cursor.adc"; Original = "$original\projects\gt5m\cursor\cursor.adc"; Name = "cursor" },
    @{ Source = "$source\projects\gt5m\detail\detail.adc"; Original = "$original\projects\gt5m\detail\detail.adc"; Name = "detail" },
    @{ Source = "$source\projects\gt5m\dialog\dialog.adc"; Original = "$original\projects\gt5m\dialog\dialog.adc"; Name = "dialog" },
    @{ Source = "$source\projects\gt5m\gtmode\gtmode.adc"; Original = "$original\projects\gt5m\gtmode\gtmode.adc"; Name = "gtmode" },
    @{ Source = "$source\projects\gt5m\install\install.adc"; Original = "$original\projects\gt5m\install\install.adc"; Name = "install" },
    @{ Source = "$source\projects\gt5m\manual\manual.adc"; Original = "$original\projects\gt5m\manual\manual.adc"; Name = "manual" },
    @{ Source = "$source\projects\gt5m\option\option.adc"; Original = "$original\projects\gt5m\option\option.adc"; Name = "option" },
    @{ Source = "$source\projects\gt5m\play_movie\play_movie.adc"; Original = "$original\projects\gt5m\play_movie\play_movie.adc"; Name = "play_movie" },
    @{ Source = "$source\projects\gt5m\race\race.adc"; Original = "$original\projects\gt5m\race\race.adc"; Name = "race" },
    @{ Source = "$source\projects\gt5m\ranking\ranking.adc"; Original = "$original\projects\gt5m\ranking\ranking.adc"; Name = "ranking" },
    @{ Source = "$source\projects\gt5m\ui_kit\ui_kit.adc"; Original = "$original\projects\gt5m\ui_kit\ui_kit.adc"; Name = "ui_kit" }
)

Write-Host "=== Comparing recompiled vs original .adc files ===" -ForegroundColor Cyan
Write-Host ""

foreach ($m in $mappings) {
    $name = $m.Name
    $htmlOut = Join-Path $outputDir "$name.html"
    
    if (-not (Test-Path $m.Source)) {
        Write-Host "  $name - MISSING (source not built)" -ForegroundColor Yellow
        continue
    }
    if (-not (Test-Path $m.Original)) {
        Write-Host "  $name - MISSING (original not found)" -ForegroundColor Yellow
        continue
    }
    
    Write-Host "  Comparing $name..." -NoNewline
    $result = python $comparePy $m.Source $m.Original $htmlOut 2>&1
    if ($LASTEXITCODE -eq 0 -and $?) {
        Write-Host " OK -> $htmlOut" -ForegroundColor Green
    } else {
        Write-Host " DONE (with notes) -> $htmlOut" -ForegroundColor Yellow
        $result | ForEach-Object { Write-Host "    $_" -ForegroundColor DarkYellow }
    }
}

Write-Host "`n=== Comparison complete ===" -ForegroundColor Cyan
Write-Host "HTML reports in: $outputDir"
