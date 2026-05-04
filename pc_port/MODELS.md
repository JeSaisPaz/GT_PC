# GT PSP Models & Assets

## Model Files

### .ad (Adhoc)
- Adhoc bytecode scripts
- Used for: course definitions, game logic
- Location: `GT.VOL/crs/*.ad`

### .cam (Camera)
- Camera setup for tracks
- Contains: camera positions, targets, transitions
- Location: `GT.VOL/crs/*.cam`

### .cinf (Course Info)
- Course metadata
- Location: `GT.VOL/crs/*.cinf`

### .envptr (Environment Pointer)
- Environment texture references

### .x (GPX/Model Data)
- 3D track models
- Location: `GT.VOL/crs/*.x`
- Likely GPX format (GT PSP model format)

### car/hq/
- High quality car models (unknown format, ~300KB each)
- Files: `00010001`, `00010002`, etc.

## File Structure (GT.VOL)

```
GT.VOL/
├── advertise/   - ads
├── car/        - car models
│   ├── hq/       high quality car models
│   ├── info/      car info
│   ├── interior/  interior models
│   ├── race/     race car models
│   └── thumbnail/ car thumbnails
├── carsound/   - car sounds
├── character/  - character models
├── crs/        - course data
│   ├── *.ad       course bytecode
│   ├── *.cam      camera
│   ├── *.cinf     course info
│   ├── *.x       3D models
│   └── *.envptr   environment pointers
├── description/- car descriptions
├── font/       - font files
├── icon/       - UI icons
├── movie/      - movies/videos
├── piece_gt5m/  - textures/images
├── products/   - product data
├── scripts/    - game scripts (Adhoc)
├── sound_gt/   - audio
├── specdb/      - database files
├── textdata/   - text/localization
└── wheel/     - wheel models
```

## TODO

- [ ] Analyze .x (GPX) model format
- [ ] Analyze car/hq/ model format
- [ ] Implement camera (.cam) loading
- [ ] Build course rendering from .x files