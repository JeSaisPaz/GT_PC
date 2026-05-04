---
tags: [pc-port, rust, specdb, database]
type: documentation
project: GT PSP PC Port
section: Core Engine
---

# SpecDB Reader — PC Port

> Database reader in Rust (`pc_port/src/engine/specdb.rs`).

## Overview

Parses GT PSP's SpecDB binary format (`.dbt` / `.idi` files).

## .DBT Format Structure

| Offset | Size | Field                                |
| ------ | ---- | ------------------------------------ |
| 0x00   | 4    | Magic: "GTDB"                        |
| 0x04   | 4    | Version + flags (0x00010008 for PSP) |
| 0x08   | 4    | Row count                            |
| 0x0C   | 4    | Column count                         |
| 0x10   | 4    | Row stride (bytes/row)               |
| 0x14   | 4    | Padding (0)                          |
| 0x18   | N*8  | Column descriptors                   |
| ...    | ...  | Row data                             |
| ...    | ...  | String block (UTF-16LE)              |

## Column Descriptor (8 bytes)

| Offset | Field |
|--------|-------|
| 0x00 | field_offset |
| 0x02 | field_bitsize (0=dynamic, 2/4/8/16/32=fixed) |
| 0x04 | reserved |

## .IDI Format Structure

| Offset | Size | Field |
|-------|------|-------|
| 0x00 | 4 | Magic: "GTID" |
| 0x04 | 4 | Key count |
| 0x08 | 4 | Padding (0) |
| 0x0C | 4 | Table ID |
| 0x10 | N*8 | Keys: (string_offset, dbt_row_id) |
| ... | ... | String table |

## Data Structures

```rust
pub struct ColumnDef {
    pub offset: u16,
    pub col_type: u16,
}

pub struct SpecDBTable {
    pub name: String,
    pub columns: Vec<ColumnDef>,
    pub stride: u32,
    pub rows: Vec<Vec<u8>>,           // Raw row data
    pub id_to_row: HashMap<u32, usize>,  // IDI lookup
    pub string_block: Vec<u8>,
    pub string_offset: u32,
    pub row_count: u32,
    pub col_count: u32,
}

pub struct SpecDB {
    pub path: String,
    pub tables: HashMap<String, SpecDBTable>,
}
```

## Key Tables

### Car Data

| Table | Rows | Description |
|-------|------|-------------|
| `GENERIC_CAR` | 837+ | Master car list |
| `ENGINE` | — | Engine specs |
| `SUSPENSION` | — | Suspension |
| `CHASSIS` | — | Chassis |
| `DRIVETRAIN` | — | Drivetrain type |
| `GEAR` | — | Gear ratios |
| `FRONTTIRE` | — | Front tire specs |
| `REARTIRE` | — | Rear tire specs |
| `BRAKE` | — | Brake specs |
| `DEFAULT_PARTS` | — | Default parts per car |
| `VARIATION` | 6042 | Color variations |

### Track & Race

| Table | Rows | Description |
|-------|------|-------------|
| `COURSE` | 85 | Track list |
| `RACE` | — | Race events |

### Strings

| Table | Description |
|-------|-------------|
| `CAR_NAME_*` | Car names (9 languages) |
| `*_StrDB` | UI strings |

## Usage

```rust
let specdb = SpecDB::new();
specdb.load_all("./specdb/GT_PSP_JP2817")?;

// Access table
let cars = specdb.get_table("GENERIC_CAR")?;

// Read car price (row 0, column offset 8)
let price = cars.read_u32(0, &cars.columns[2])?;

// Read car name
let name = cars.car_name_by_row(0)?;
```

## Parser Methods

### Table Access

```rust
pub fn get_table(&self, name: &str) -> Option<&SpecDBTable>
pub fn load_all(&mut self, path: &str) -> Result<(), String>
```

### Row Reading

```rust
pub fn read_u32(&self, row: usize, col: &ColumnDef) -> u32
pub fn read_i32(&self, row: usize, col: &ColumnDef) -> i32
pub fn read_u16(&self, row: usize, col: &ColumnDef) -> u16
```

### Car-Specific Accessors

```rust
pub fn car_default_parts(&self, row: usize) -> u32
pub fn car_price(&self, row: usize) -> u32
pub fn car_year(&self, row: usize) -> u16
pub fn car_power_max(&self, row: usize) -> u32
pub fn car_power_min(&self, row: usize) -> u32
pub fn car_maker(&self, row: usize) -> u8
```

## Integration

Registered in `engine::gtengine.rs`:

```rust
registry.register("main,gtengine,MSpecDB,initialize", |_| {
    let path = format!("{}\\specdb\\GT_PSP_JP2817", assets_root());
    specdb.load_all(&path)
})
```

## See Also

- [[40_Reference/00_Index|Reference Index]]
- [[10_PC_Port/00_Index|PC Port Index]]