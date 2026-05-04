# GT PSP Textures

## File Formats

### IMG Format (PSP Texture)
- Magic: `3SXT` or `TXS3`
- Located in: `GT.VOL/piece_gt5m/*`
- Used for: course images, logos, menus, backgrounds

### Supported Pixel Formats

| ID | Format | Description | BPP |
|---|--------|-------------|-----|
| 0x01 | RGBA8888 | 32-bit RGBA | 4 |
| 0x03 | RGB5A1 | 16-bit RGB + 1-bit alpha | 2 |
| 0x04 | RGB565 | 16-bit RGB | 2 |
| 0x05 | RGBA4444 | 16-bit RGBA | 2 |
| 0x07 | L8 | 8-bit luminance | 1 |
| 0x08 | L4 | 4-bit luminance | 0.5 |
| 0x0A | DXT1 | compressed (unsupported) | - |

## Texture Locations

### piece_gt5m/ (Graphics Pack)
- `course_image/` - course preview images
- `course_logo_S/` - small course logos
- `course_logo_SS/` - very small course logos  
- `course_map_menu/` - map menu images
- `course_map_race/` - race map images
- `license_bg/` - license backgrounds
- `env/` - environment textures
- `mission_flyer/` - mission images
- `tunner_logo_*/` - manufacturer logos

### icon/
- Car icons, UI icons

### font/
- Font data (requires GPB loading)

## Texture Dimensions

Common sizes:
- 96x64 (S logo)
- 128x64 (SS logo)
- 256x128 (menu backgrounds)
- 480x272 (full screen)
- 512x256 (2D elements)
- 480x272 PSP native

## Loading

```rust
use crate::platform::texture::Texture;

let tex = Texture::from_img_file("path/to/texture.img")?;
```

## TODO

- [ ] Implement GPB texture bank format
- [ ] Add texture caching system
- [ ] Support mipmaps (if present)
- [ ] Handle swizzled/un-swizzled formats