# GTPSP Asset Conversion Scripts

This directory contains Python scripts for converting Gran Turismo PSP game assets between proprietary formats and standard editable formats.

## Scripts Overview

### 1. `convert_audio.py`
Converts GT PSP audio files to standard formats.

**Features:**
- Converts `.at3` (ATRAC3) files to `.wav`
- Analyzes audio file structure and metadata
- Supports batch conversion
- Identifies different audio file types

**Usage:**
```bash
# Analyze all audio files
python convert_audio.py analyze --input "../files/decompiled"

# Convert AT3 files to WAV
python convert_audio.py convert --input "../files/decompiled" --output "../converted/audio"

# Test conversion with a single file
python convert_audio.py test --input "../files/decompiled"
```

**Requirements:**
- `ffmpeg` for audio conversion (must be in PATH)
- Python 3.x

### 2. `convert_textures.py`
Converts GT PSP texture files to PNG format.

**Features:**
- Converts `.img` (TXS3) files to `.png`
- Handles non-standard texture dimensions
- Supports RGB565, RGBA5551, RGBA8888 formats
- Includes round-trip testing
- Batch conversion support

**Usage:**
```bash
# Analyze all texture files
python convert_textures.py analyze --input "../files/decompiled"

# Convert IMG files to PNG
python convert_textures.py convert --input "../files/decompiled" --output "../converted/textures"

# Test round-trip conversion
python convert_textures.py roundtrip --input "../files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/tunner_logo_S/audi.img"
```

**Requirements:**
- Python 3.x with Pillow library: `pip install Pillow`
- numpy: `pip install numpy`

### 3. `convert_all.bat`
Windows batch script for converting all assets at once.

**Usage:**
```bash
convert_all.bat
```

This script will:
1. Analyze audio and texture files
2. Convert AT3 files to WAV
3. Convert IMG files to PNG
4. Test round-trip conversion
5. Organize output in `../converted/` directory

### 4. `debug_texture.py`
Debug tool for analyzing texture file structure.

**Usage:**
```bash
python debug_texture.py
```

### 5. `test_texture_conversion.py`
Test tool for trying different texture decoding methods.

**Usage:**
```bash
python test_texture_conversion.py
```

## File Format Details

### Audio Files (`.at3`)
- Format: RIFF/WAVE with ATRAC3 codec
- Sample rate: 44.1kHz
- Channels: 2 (stereo)
- Location: `sound_gt/track/` (music), `sound_gt/se/` (sound effects)

### Texture Files (`.img`)
- Format: TXS3 with `3SXT` magic (little-endian)
- Common dimensions: 16x364, 32x182, 64x91 (non-standard)
- Pixel format: Mostly RGB565 (2 bytes per pixel)
- Location: `piece_gt5m/tunner_logo_S/` (manufacturer logos)

### Sound Banks (`.sgd`)
- Format: Sony SGD (Sound Group Data)
- Contains multiple sound effects
- Needs further reverse engineering

### Car Sounds (`carsound/`)
- Binary format without extension
- Per-car engine audio
- Needs further analysis

## Conversion Workflow

### For Modders:
1. Extract game with `GTPSPVolTools.exe`
2. Convert assets using these scripts
3. Edit converted files (WAV/PNG)
4. Convert back to game format
5. Repack with `GTPSPVolTools.exe`

### For Researchers:
1. Use analysis commands to study file formats
2. Examine converted assets to understand game content
3. Document findings for the modding community

## Dependencies Installation

```bash
# Install Python dependencies
pip install Pillow numpy

# Install ffmpeg (for audio conversion)
# Windows: Download from https://ffmpeg.org/download.html
# Add ffmpeg to your PATH
```

## Known Issues

1. **ATRAC3 Encoding**: Converting WAV back to AT3 requires proprietary Sony encoder
2. **SGD Format**: Sound bank extraction not fully implemented
3. **Texture Headers**: Some TXS3 headers have unusual values that need manual interpretation
4. **Car Sounds**: Engine audio format needs reverse engineering

## Contributing

Feel free to:
- Report bugs or issues
- Improve format detection algorithms
- Add support for additional file formats
- Create GUI interfaces for the tools
- Document new discoveries about file formats

## License

These scripts are provided as-is for educational and modding purposes. They are part of the GTPSP Decompilation Project.