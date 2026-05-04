# GT PC — Gran Turismo PSP Native PC Port

Native PC port of Gran Turismo PSP using a custom Rust Adhoc VM + SDL2/OpenGL.

## Structure

| Directory | Description |
|-----------|-------------|
| `pc_port/` | Main PC port — Rust VM, race engine, graphics (SDL2 + OpenGL) |
| `freecam/` | Standalone GT PSP freecam viewer (Rust + wgpu/Vulkan) |
| `source/` | Reconstructed Adhoc source code (`.ad` files, from OpenAdhoc) |
| `mod_loader/` | GT PSP Mod Loader framework for PPSSPP |
| `moddingTools/` | Python modding utilities (texture patching, VOL tools) |
| `scripts/` | Build/conversion utility scripts |
| `workflow/` | Ghidra CLI bridge (Rust) and decompilation tooling |
| `docs/` | Project documentation and Obsidian knowledge vault |

## Quick Start

```bash
# PC Port
cd pc_port
cargo build --release
cargo run --release -- --boot

# Freecam
cd freecam
cargo run --release
```

## Documentation

Full documentation is in `docs/`:
- `PROJECT.md` — Master project documentation
- `PC_PORT.md` — PC port design and architecture
- `SCRIPTS_ARCHITECTURE.md` — Adhoc script reference
- `3LDM.md` — 3D model format specification
- `vault/` — Obsidian knowledge base (detailed technical notes)

## Requirements

- Rust (edition 2024)
- SDL2 development libraries
- OpenGL 3.3+ (for PC port)
- Vulkan (for freecam)

The freecam references PPSSPP source code for texture format research. To build with full texture decoding:
```bash
cd freecam/source
git clone --depth 1 https://github.com/hrydgard/ppsspp.git PPSSPP
```

## Third-Party Projects & Credits

This project builds upon and incorporates work from the following projects:

### Core Reverse Engineering
| Project | Author | License | Usage |
|---------|--------|---------|-------|
| [OpenAdhoc](https://github.com/Nenkai/OpenAdhoc) | Nenkai / pez2k | GPL-3.0 | Reconstructed `.ad` source code in `source/` |
| [GTAdhocToolchain](https://github.com/Nenkai/GTAdhocToolchain) | Nenkai | MIT | Adhoc compiler/decompiler used to produce disassemblies |
| [GTPSPVolTools](https://github.com/Nenkai/GTPSPVolTools) | Nenkai | MIT | GT.VOL extraction used to unpack game assets |
| [GT-File-Specifications-Documentation](https://github.com/Nenkai/GT-File-Specifications-Documentation) | Nenkai | MIT | Binary format specifications (3LDM, TXS3, SpecDB) |
| [GT Modding Hub](https://nenkai.github.io/gt-modding-hub/) | Nenkai | — | Community documentation and reference |

### Emulation & Testing
| Project | Author | License | Usage |
|---------|--------|---------|-------|
| [PPSSPP](https://github.com/hrydgard/ppsspp) | Henrik Rydgård | GPL-2.0 | PSP emulator for testing and reverse engineering reference |
| [Ghidra](https://github.com/NationalSecurityAgency/ghidra) | NSA | Apache-2.0 | Reverse engineering framework for EBOOT.BIN analysis |

### Rust Dependencies (pc_port)
| Crate | Version | License | Usage |
|-------|---------|---------|-------|
| [sdl2](https://github.com/Rust-SDL2/rust-sdl2) | 0.37 | MIT | SDL2 bindings for windowing and input |
| [gl](https://github.com/brendanzab/gl-rs) | 0.14 | Apache-2.0/MIT | OpenGL bindings for GPU rendering |
| [ab_glyph](https://github.com/alexheretic/ab-glyph) | 0.2 | Apache-2.0/MIT | Font rasterization for HUD text |
| [rand](https://github.com/rust-random/rand) | 0.8 | MIT/Apache-2.0 | Random number generation |

### Rust Dependencies (freecam)
| Crate | Version | License | Usage |
|-------|---------|---------|-------|
| [wgpu](https://github.com/gfx-rs/wgpu) | 29 | MIT/Apache-2.0 | GPU abstraction (Vulkan/Metal/DX12) |
| [winit](https://github.com/rust-windowing/winit) | 0.30 | Apache-2.0 | Cross-platform window creation |
| [glam](https://github.com/bitshifter/glam-rs) | 0.26 | MIT/Apache-2.0 | 3D math (vectors, matrices) |
| [image](https://github.com/image-rs/image) | 0.25 | MIT/Apache-2.0 | Image decoding for textures |

### Additional Tools
| Tool | Author | Usage |
|------|--------|-------|
| GT2TextureEditor | pez2k | GT texture format analysis |
| GT3PMBDumper | pez2k | Model format research |
| GTSpecDB | pez2k | SpecDB database tooling |
| GTMusicInfEditor | pez2k | Audio metadata editing |

## License

This project's original code is provided as-is. The reconstructed Adhoc source (`source/`) is licensed under GPL-3.0 as part of OpenAdhoc. See individual components for their respective licenses.

## Status

Active development. See `docs/PROJECT.md` for detailed status.
