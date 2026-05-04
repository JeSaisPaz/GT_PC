---
tags: [reference, specdb, database]
type: reference
game: gt-psp
---

# Reference — Index

> Game reference data: SpecDB tables, archives, scripts.

## SpecDB Database

Located in `files/decompiled/GT.VOL/specdb/GT_PSP_JP2817/`

| Table | Rows | Description |
|-------|------|-------------|
| GENERIC_CAR.dbt | 837+ | Car specifications |
| ENGINE.dbt | — | Engine data |
| SUSPENSION.dbt | — | Suspension |
| CHASSIS.dbt | — | Chassis |
| DRIVETRAIN.dbt | — | Drivetrain |
| GEAR.dbt | — | Gear ratios |
| FRONTTIRE.dbt | — | Front tires |
| REARTIRE.dbt | — | Rear tires |
| BRAKE.dbt | — | Brakes |
| VARIATION.dbt | 6042 | Color variations |
| COURSE.dbt | 85 | Track specs |
| RACE.dbt | — | Race events |
| CAR_NAME_*.dbt | — | Localized names (9 langs) |

## Game Content

| Category | Count |
|----------|-------|
| Cars | 831+ |
| Tracks | 45 unique |
| Variations | 6042 |
| Languages | 9 |

## GT.VOL Structure

```
GT.VOL/
├── scripts/gt5m/       # ~18 ADC files in PC port assets
├── projects/gt5m/      # UI projects
├── products/gt5m/      # Menu classes
├── car/                 # 831+ cars
│   ├── hq/           # High quality models
│   ├── race/          # Race models
│   ├── thumbnail/    # Preview images
│   └── info/         # Metadata
├── crs/                # Tracks (45)
├── specdb/             # 123 tables
├── textdata/           # XML configs
├── piece_gt5m/         # UI textures
├── sound_gt/           # Music/SFX
└── font/             # Fonts
```

## Core Scripts

| Script | Purpose |
|--------|---------|
| Application.adc | Entry |
| bootstrap.adc | Init |
| packed_main_loop.adc | Main loop |
| bootstrap_phase2.adc | Phase 2 |
| init_sound.adc | Audio |
| shutdown.adc | Cleanup |

## Adhoc Version

- **Version**: 12
- **Used by**: GT PSP, GT5, GT6, GT Sport

## See Also

- [[10_PC_Port/00_Index|PC Port Index]]
- [[20_ADHOC_VM/00_Index|Adhoc VM]]
- [[30_Technical/00_Index|Technical]]