#!/usr/bin/env python3
"""
GTPSP Audio Conversion Tool
Converts GT PSP audio files between game formats and standard formats.

Supported formats:
- .at3 (ATRAC3) ↔ .wav/.mp3/.ogg
- .sgd (Sony sound banks) → extract individual sounds
- Car engine sounds (binary) → analyze and convert

Requirements:
- ffmpeg (for audio conversion)
- Python libraries: wave, struct, os, sys, subprocess
"""

import os
import sys
import struct
import subprocess
import wave
from pathlib import Path
from typing import Dict, List, Optional, Tuple

class AT3Converter:
    """Converter for ATRAC3 (.at3) audio files"""
    
    @staticmethod
    def is_at3_file(filepath: str) -> bool:
        """Check if file is ATRAC3 format"""
        try:
            with open(filepath, 'rb') as f:
                header = f.read(12)
                return header[0:4] == b'RIFF' and header[8:12] == b'WAVE'
        except:
            return False
    
    @staticmethod
    def convert_to_wav(at3_path: str, wav_path: str) -> bool:
        """
        Convert AT3 to WAV using ffmpeg
        Returns True if successful
        """
        try:
            # Check if ffmpeg is available
            result = subprocess.run(['ffmpeg', '-version'], capture_output=True)
            if result.returncode != 0:
                print("Error: ffmpeg not found. Please install ffmpeg.")
                return False
            
            # Convert AT3 to WAV
            cmd = [
                'ffmpeg', '-i', at3_path,
                '-acodec', 'pcm_s16le',
                '-ar', '44100',
                '-ac', '2',
                wav_path,
                '-y'  # Overwrite output file
            ]
            
            print(f"Converting {os.path.basename(at3_path)} to WAV...")
            result = subprocess.run(cmd, capture_output=True, text=True)
            
            if result.returncode == 0:
                print(f"Successfully converted to {wav_path}")
                return True
            else:
                print(f"Conversion failed: {result.stderr}")
                return False
                
        except Exception as e:
            print(f"Error during conversion: {e}")
            return False
    
    @staticmethod
    def convert_to_at3(wav_path: str, at3_path: str) -> bool:
        """
        Convert WAV back to AT3 format
        Note: This requires ATRAC3 encoder which may not be available
        """
        print("Warning: ATRAC3 encoding not fully implemented")
        print("ATRAC3 encoder is proprietary Sony technology")
        print("Consider using alternative formats for modding")
        return False
    
    @staticmethod
    def analyze_file(filepath: str) -> Dict:
        """Analyze AT3 file structure"""
        info = {
            'filename': os.path.basename(filepath),
            'size': os.path.getsize(filepath),
            'is_at3': False,
            'channels': 0,
            'sample_rate': 0,
            'duration': 0
        }
        
        try:
            with open(filepath, 'rb') as f:
                # Read RIFF header
                riff = f.read(12)
                if riff[0:4] != b'RIFF':
                    return info
                
                info['is_at3'] = True
                
                # Skip to fmt chunk
                f.seek(12)
                while True:
                    chunk_header = f.read(8)
                    if len(chunk_header) < 8:
                        break
                    
                    chunk_id = chunk_header[0:4]
                    chunk_size = struct.unpack('<I', chunk_header[4:8])[0]
                    
                    if chunk_id == b'fmt ':
                        fmt_data = f.read(min(chunk_size, 256))
                        if len(fmt_data) >= 16:
                            # Parse basic WAVEFORMATEX
                            audio_format = struct.unpack('<H', fmt_data[0:2])[0]
                            info['channels'] = struct.unpack('<H', fmt_data[2:4])[0]
                            info['sample_rate'] = struct.unpack('<I', fmt_data[4:8])[0]
                            byte_rate = struct.unpack('<I', fmt_data[8:12])[0]
                            block_align = struct.unpack('<H', fmt_data[12:14])[0]
                            
                            # Calculate approximate duration
                            if byte_rate > 0:
                                info['duration'] = info['size'] / byte_rate
                        
                        break
                    else:
                        # Skip this chunk
                        f.seek(chunk_size, 1)
        
        except Exception as e:
            print(f"Error analyzing file: {e}")
        
        return info

class SGDConverter:
    """Converter for Sony SGD sound bank files"""
    
    @staticmethod
    def is_sgd_file(filepath: str) -> bool:
        """Check if file is SGD format"""
        try:
            with open(filepath, 'rb') as f:
                header = f.read(4)
                return header == b'SGXD'
        except:
            return False
    
    @staticmethod
    def analyze_file(filepath: str) -> Dict:
        """Analyze SGD file structure"""
        info = {
            'filename': os.path.basename(filepath),
            'size': os.path.getsize(filepath),
            'is_sgd': False,
            'sound_count': 0,
            'regions': []
        }
        
        try:
            with open(filepath, 'rb') as f:
                # Check SGXD header
                header = f.read(4)
                if header != b'SGXD':
                    return info
                
                info['is_sgd'] = True
                
                # Read SGXD chunk size
                sgxd_size = struct.unpack('<I', f.read(4))[0]
                
                # Look for RGND (Region Data) chunk
                f.seek(0)
                data = f.read(min(1024, info['size']))
                
                # Find RGND chunks
                pos = 0
                while pos < len(data) - 8:
                    if data[pos:pos+4] == b'RGND':
                        region_size = struct.unpack('<I', data[pos+4:pos+8])[0]
                        info['regions'].append({
                            'offset': pos,
                            'size': region_size
                        })
                        pos += 8 + region_size
                    else:
                        pos += 1
                
                info['sound_count'] = len(info['regions'])
        
        except Exception as e:
            print(f"Error analyzing SGD file: {e}")
        
        return info
    
    @staticmethod
    def extract_sounds(sgd_path: str, output_dir: str) -> bool:
        """
        Extract sounds from SGD file
        This is a basic implementation - actual extraction depends on SGD format
        """
        print(f"Warning: SGD extraction not fully implemented")
        print(f"SGD files contain multiple sounds in proprietary format")
        print(f"Further reverse engineering needed for proper extraction")
        return False

class CarSoundConverter:
    """Converter for car engine sound files"""
    
    @staticmethod
    def analyze_file(filepath: str) -> Dict:
        """Analyze car engine sound file"""
        info = {
            'filename': os.path.basename(filepath),
            'size': os.path.getsize(filepath),
            'is_car_sound': False,
            'format': 'unknown'
        }
        
        try:
            # Car sound files are numbered (e.g., 00001, 10030, etc.)
            # They're likely raw audio data or compressed format
            with open(filepath, 'rb') as f:
                # Read first 64 bytes for analysis
                header = f.read(64)
                
                # Check for common audio headers
                if header[0:4] == b'RIFF':
                    info['format'] = 'RIFF/WAVE'
                elif header[0:4] == b'OggS':
                    info['format'] = 'OGG'
                elif header[0:2] == b'\xFF\xFB' or header[0:3] == b'ID3':
                    info['format'] = 'MP3'
                else:
                    # Try to guess from file size patterns
                    file_size = info['size']
                    
                    # Common audio durations at common bitrates
                    # 44.1kHz stereo 16-bit = 176400 bytes/sec
                    for duration in [1, 2, 3, 5, 10, 30]:
                        expected_size = duration * 176400
                        if abs(file_size - expected_size) < 1000:
                            info['format'] = f'Raw PCM ~{duration}s'
                            break
                
                info['is_car_sound'] = True
        
        except Exception as e:
            print(f"Error analyzing car sound: {e}")
        
        return info

def batch_convert_at3_to_wav(input_dir: str, output_dir: str, recursive: bool = True):
    """Convert all AT3 files in directory to WAV"""
    input_path = Path(input_dir)
    output_path = Path(output_dir)
    output_path.mkdir(parents=True, exist_ok=True)
    
    converted = 0
    failed = 0
    
    # Find all AT3 files
    pattern = "**/*.at3" if recursive else "*.at3"
    
    for at3_file in input_path.glob(pattern):
        # Create output path
        rel_path = at3_file.relative_to(input_path)
        wav_file = output_path / rel_path.with_suffix('.wav')
        wav_file.parent.mkdir(parents=True, exist_ok=True)
        
        # Convert
        if AT3Converter.convert_to_wav(str(at3_file), str(wav_file)):
            converted += 1
        else:
            failed += 1
    
    print(f"\nConversion complete:")
    print(f"  Converted: {converted}")
    print(f"  Failed: {failed}")
    return converted, failed

def analyze_audio_files(input_dir: str, recursive: bool = True):
    """Analyze all audio files in directory"""
    input_path = Path(input_dir)
    
    print("Audio File Analysis Report")
    print("=" * 80)
    
    # Check for AT3 files
    at3_files = list(input_path.glob("**/*.at3" if recursive else "*.at3"))
    if at3_files:
        print(f"\nAT3 Files ({len(at3_files)}):")
        for at3_file in at3_files[:10]:  # Show first 10
            info = AT3Converter.analyze_file(str(at3_file))
            if info['is_at3']:
                print(f"  {at3_file.name}: {info['size']:,} bytes, "
                      f"{info['channels']}ch, {info['sample_rate']}Hz, "
                      f"~{info['duration']:.1f}s")
        
        if len(at3_files) > 10:
            print(f"  ... and {len(at3_files) - 10} more")
    
    # Check for SGD files
    sgd_files = list(input_path.glob("**/*.sgd" if recursive else "*.sgd"))
    if sgd_files:
        print(f"\nSGD Files ({len(sgd_files)}):")
        for sgd_file in sgd_files:
            info = SGDConverter.analyze_file(str(sgd_file))
            if info['is_sgd']:
                print(f"  {sgd_file.name}: {info['size']:,} bytes, "
                      f"{info['sound_count']} sound regions")
    
    # Check for car sound files (no extension)
    car_sound_dirs = []
    for item in input_path.glob("**/carsound/**" if recursive else "carsound/*"):
        if item.is_file() and not item.suffix and item.name.isdigit():
            car_sound_dirs.append(item.parent)
    
    if car_sound_dirs:
        car_sound_dirs = list(set(car_sound_dirs))
        print(f"\nCar Sound Directories ({len(car_sound_dirs)}):")
        for dir_path in car_sound_dirs[:5]:
            files = list(dir_path.glob("*"))
            print(f"  {dir_path.relative_to(input_path)}: {len(files)} files")
            
            # Analyze first file
            if files:
                first_file = files[0]
                info = CarSoundConverter.analyze_file(str(first_file))
                print(f"    Sample: {first_file.name} - {info['format']}, "
                      f"{info['size']:,} bytes")

def main():
    """Main function"""
    import argparse
    
    parser = argparse.ArgumentParser(description='GTPSP Audio Conversion Tool')
    parser.add_argument('command', choices=['convert', 'analyze', 'test'],
                       help='Command to execute')
    parser.add_argument('--input', '-i', default='files/decompiled',
                       help='Input directory')
    parser.add_argument('--output', '-o', default='converted/audio',
                       help='Output directory for conversions')
    parser.add_argument('--recursive', '-r', action='store_true', default=True,
                       help='Process directories recursively')
    parser.add_argument('--no-recursive', action='store_false', dest='recursive',
                       help='Do not process directories recursively')
    
    args = parser.parse_args()
    
    if args.command == 'convert':
        print("Converting AT3 files to WAV...")
        batch_convert_at3_to_wav(args.input, args.output, args.recursive)
    
    elif args.command == 'analyze':
        analyze_audio_files(args.input, args.recursive)
    
    elif args.command == 'test':
        print("Testing audio conversion tools...")
        
        # Test with a sample AT3 file
        test_files = list(Path(args.input).glob("**/*.at3"))
        if test_files:
            test_file = str(test_files[0])
            print(f"\nTesting with: {test_file}")
            
            # Analyze
            if AT3Converter.is_at3_file(test_file):
                info = AT3Converter.analyze_file(test_file)
                print(f"Analysis: {info}")
                
                # Test conversion
                output_dir = Path(args.output) / "test"
                output_dir.mkdir(parents=True, exist_ok=True)
                wav_file = output_dir / Path(test_file).name.replace('.at3', '.wav')
                
                print(f"\nAttempting conversion to: {wav_file}")
                success = AT3Converter.convert_to_wav(test_file, str(wav_file))
                print(f"Conversion successful: {success}")
            else:
                print("Not a valid AT3 file")
        else:
            print("No AT3 files found for testing")

if __name__ == '__main__':
    main()