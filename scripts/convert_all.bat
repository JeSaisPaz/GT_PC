@echo off
REM GTPSP Asset Conversion Batch Script
REM Converts all game assets to editable formats

echo ========================================
echo GTPSP Asset Conversion Tool
echo ========================================

REM Set paths
set PROJECT_DIR=%~dp0..
set SCRIPTS_DIR=%PROJECT_DIR%\scripts
set CONVERTED_DIR=%PROJECT_DIR%\converted

REM Create output directories
mkdir "%CONVERTED_DIR%\audio" 2>nul
mkdir "%CONVERTED_DIR%\textures" 2>nul
mkdir "%CONVERTED_DIR%\models" 2>nul

echo.
echo 1. Analyzing audio files...
python "%SCRIPTS_DIR%\convert_audio.py" analyze --input "%PROJECT_DIR%\files\decompiled"

echo.
echo 2. Converting audio files (AT3 to WAV)...
python "%SCRIPTS_DIR%\convert_audio.py" convert --input "%PROJECT_DIR%\files\decompiled" --output "%CONVERTED_DIR%\audio"

echo.
echo 3. Analyzing texture files...
python "%SCRIPTS_DIR%\convert_textures.py" analyze --input "%PROJECT_DIR%\files\decompiled"

echo.
echo 4. Converting texture files (IMG to PNG)...
python "%SCRIPTS_DIR%\convert_textures.py" convert --input "%PROJECT_DIR%\files\decompiled" --output "%CONVERTED_DIR%\textures"

echo.
echo 5. Testing round-trip conversion...
python "%SCRIPTS_DIR%\convert_textures.py" roundtrip --input "%PROJECT_DIR%\files\decompiled\Gran Turismo\PSP_GAME\USRDIR\GT.VOL\piece_gt5m\tunner_logo_S\audi.img"

echo.
echo ========================================
echo Conversion Complete!
echo ========================================
echo.
echo Converted files are in: %CONVERTED_DIR%
echo.
echo Audio:   %CONVERTED_DIR%\audio\
echo Textures: %CONVERTED_DIR%\textures\
echo.
pause