---
tags: [specdb, database, optimization, loading, cache]
type: documentation
project: GT PSP PC Port
section: Technical
---

# SpecDB Loading

> Detailed process of loading GT PSP's binary database tables, with optimization strategies.

## Overview

SpecDB (Specification Database) contains all car/track/race data:
- ~45+ tables
- 837 cars, 85 tracks, 6000+ color variations
- Binary format: `.dbt` + optional `.idi` index

---

## File Format

### .dbt (Data Table)

```
Offset  Size  Field
0x00    4     Magic: "GTDB"
0x04    4     Version + flags (u32 LE, 0x00010008 for PSP)
0x08    4     Row count (u32)
0x0C    4     Column count (u32)
0x10    4     Row stride (bytes per row)
0x14    4     Padding (0)
0x18    N*8   Column descriptors (8 bytes each)
...           Row data (row_count × stride bytes)
...           String data (4-byte length prefix, then UTF-16LE)
```

### Column Descriptor (8 bytes)

```
u16 field_offset    // Byte offset within row
u16 field_bitsize // 0=dynamic, 2/4/8/16/32 = fixed width
u32 reserved
```

### .idi (Index Table)

```
Offset  Size  Field
0x00    4     Magic: "GTID"
0x04    4     Key count
0x08    4     Padding (0)
0x0C    4     Table ID
0x10    N*8   Keys: (string_offset: u32, dbt_row_id: u32)
...           String table
```

---

## Loading Process

### SpecDB Structure

```rust
pub struct SpecDB {
    pub tables: HashMap<String, SpecDBTable>,
    pub base_path: String,
}

pub struct SpecDBTable {
    pub name: String,
    pub columns: Vec<ColumnDef>,      // Column descriptors
    pub stride: u32,               // Bytes per row
    pub rows: Vec<Vec<u8>>,        // Raw row data (cloned)
    pub id_to_row: HashMap<u32, usize>,  // ID → row index
    pub string_block: Vec<u8>,    // UTF-16LE strings
    pub string_offset: u32,       // String block start
    pub row_count: u32,
    pub col_count: u32,
}
```

### Load All Tables

```rust
pub fn load_all(&mut self, path: &str) -> Result<(), String> {
    self.base_path = path.to_string();
    let dir = Path::new(path);
    
    if !dir.is_dir() {
        return Err(format!("SpecDB path not found: {}", path));
    }
    
    let mut loaded = 0u32;
    let mut failed = 0u32;
    
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let fname = entry.file_name().to_string_lossy().to_string();
        
        if fname.ends_with(".dbt") {
            let name = fname.trim_end_matches(".dbt");
            match self.load_table(name) {
                Ok(_) => { loaded += 1; }
                Err(e) => { failed += 1; }
            }
        }
    }
    
    eprintln!("SpecDB: loaded {} tables, {} failed", loaded, failed);
    Ok(())
}
```

### Load Single Table

```rust
pub fn load_table(&mut self, table_name: &str) -> Result<(), String> {
    let dbt_path = format!("{}/{}.dbt", self.base_path, table_name);
    let idi_path = format!("{}/{}.idi", self.base_path, table_name);
    
    let dbt_data = fs::read(&dbt_path)?;
    let idi_data = fs::read(&idi_path).ok();  // Optional
    
    let table = SpecDBTable::from_files(&dbt_data, idi_data.as_deref(), table_name)?;
    self.tables.insert(table_name.to_string(), table);
    
    Ok(())
}
```

### Table Parsing

```rust
pub fn from_files(dbt_data: &[u8], idi_data: Option<&[u8]>, table_name: &str) -> Result<Self, String> {
    // Validate header
    if dbt_data.len() < 24 {
        return Err("DBT file too short".to_string());
    }
    if &dbt_data[0..4] != b"GTDB" {
        return Err(format!("Bad DBT magic: {:02X?}", &dbt_data[0..4]));
    }
    
    let row_count = read_u32_le(dbt_data, 0x08);
    let col_count = read_u32_le(dbt_data, 0x0C);
    let stride = read_u32_le(dbt_data, 0x10);
    
    // Parse column descriptors (0x18 + col*8)
    let col_desc_start = 0x18u32;
    let mut columns = Vec::new();
    for i in 0..actual_cols {
        let pos = (col_desc_start + i * 8) as usize;
        let col_off = read_u16_le(dbt_data, pos);
        let col_type = read_u16_le(dbt_data, pos + 2);
        columns.push(ColumnDef { offset: col_off, col_type });
    }
    
    // Parse row data
    let col_desc_end = col_desc_start + actual_cols * 8;
    let row_data_start = col_desc_end;
    
    let mut rows: Vec<Vec<u8>> = Vec::new();
    for i in 0..actual_row_count {
        let row_off = (row_data_start + i * stride) as usize;
        rows.push(dbt_data[row_off..row_off + stride as usize].to_vec());
    }
    
    // String block
    let str_start = row_data_start + row_count * stride;
    let string_block = dbt_data[str_start as usize..].to_vec();
    
    // Parse IDI index
    let mut id_to_row = HashMap::new();
    if let Some(idi) = idi_data {
        if idi.len() >= 16 && &idi[0..4] == b"GTID" {
            let idi_count = read_u32_le(idi, 0x04) as usize;
            for i in 0..idi_count {
                let entry_pos = 0x10 + i * 8;
                let row_id = read_u32_le(idi, entry_pos + 4);
                id_to_row.insert(row_id, i);
            }
        }
    }
    
    Ok(SpecDBTable { ... })
}
```

---

## Key Tables

| Table | Rows | Purpose |
|-------|------|---------|
| `GENERIC_CAR` | 837 | Car base specs |
| `ENGINE` | ~850 | Engine specs |
| `CHASSIS` | ~800 | Chassis specs |
| `DEFAULT_PARTS` | ~850 | Default car parts |
| `DRIVETRAIN` | 4 | Drive types |
| `DISPLACEMENT` | ~50 | Engine sizes |
| `COURSE` | 85 | Tracks |
| `VARIATION` | 6042 | Car colors |
| `RACE` | ~500 | Race events |
| `CAR_NAME_american` | 837 | English names |
| `CAR_NAME_japanese` | 837 | Japanese names |

---

## Access Patterns

### Direct Row Access

```rust
// O(1) when row index known
table.rows[row_idx]          // Direct slice access
table.read_column_value(row_idx, &column)
```

### ID Lookup (via .idi)

```rust
// O(1) via hash map
pub fn get_row_by_id(&self, id: u32) -> Option<usize> {
    self.id_to_row.get(&id).copied()
}
```

### String Access

```rust
pub fn read_string(&self, offset: u32) -> String {
    let off = offset as usize;
    let sb = &self.string_block;
    
    let byte_len = read_u32_le(sb, off) as usize;
    let u16_data: Vec<u16> = sb[off + 4..off + 4 + byte_len]
        .chunks(2)
        .filter(|c| c.len() == 2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    
    String::from_utf16_lossy(&u16_data).trim_end_matches(char::from(0)).to_string()
}
```

---

## Native API Registration

### Registration Pattern

```rust
pub fn register_gtengine(registry: &mut NativeRegistry, specdb: Rc<RefCell<SpecDB>>) {
    // Each accessor clones the SpecDB Rc
    let s = specdb.clone();
    
    registry.register("main,gtengine,MSpecDB,getCarPrice", Rc::new(move |args: &[Value]| {
        let idx = args.first().and_then(|a| a.as_i32()).unwrap_or(0) as usize;
        
        let sd = s.borrow();
        if let Some(t) = sd.get_table("GENERIC_CAR") {
            if idx < t.row_count() {
                let d = &t.rows[idx];
                let v = read_u32_le(d, 4) as i32;  // Price at offset 4
                return Value::Int(v);
            }
        }
        Value::Int(0)
    }));
}
```

### Helper Functions for Relations

```rust
// Car → Engine (via DEFAULT_PARTS)
fn car_to_engine(sd: &SpecDB, car_idx: usize) -> Option<usize> {
    let car = sd.get_table("GENERIC_CAR")?;
    let dp_id = car.car_default_parts(car_idx);  // DEFAULT_PARTS row ID
    
    let dp = sd.get_table("DEFAULT_PARTS")?;
    let dp_row = dp.get_row_by_id(dp_id)?;  // ID → row index
    
    let eng_id = dp.read_column_value(dp_row, &dp.columns[11]);  // ENGINE ID
    let eng = sd.get_table("ENGINE")?;
    eng.get_row_by_id(eng_id)
}

// Usage in native function:
registry.register("main,gtengine,MSpecDB,getCarCatalogPs", Rc::new(move |args| {
    let idx = args.first().and_then(|a| a.as_i32()).unwrap_or(0) as usize;
    let sd = s.borrow();
    
    if let Some(eng_row) = car_to_engine(&sd, idx) {
        if let Some(t) = sd.get_table("ENGINE") {
            return Value::Int(t.engine_ps_value(eng_row) as i32);
        }
    }
    Value::Int(200)  // Default
}));
```

---

## Optimization Strategies

### 1. Pre-allocation

```rust
// When row count known:
let mut rows: Vec<Vec<u8>> = Vec::with_capacity(row_count as usize);

// When column count known:
let mut columns = Vec::with_capacity(actual_cols as usize);
```

### 2. Bounds Checking with Saturating Math

```rust
// Prevent OOM from malformed files
let max_rows = (dbt_data.len().saturating_sub(row_data_start as usize)) / row_bytes.max(1);
let actual_row_count = (row_count as usize).min(max_rows);
```

### 3. Lazy String Decoding

Strings are UTF-16LE with 4-byte length prefix:

```rust
// Only decode when accessed
pub fn read_string(&self, offset: u32) -> String {
    // Read length prefix
    let byte_len = read_u32_le(sb, off) as usize;
    
    // Decode UTF-16LE → String
    let u16_data: Vec<u16> = ...;
    String::from_utf16_lossy(&u16_data)
}
```

### 4. Row Cloning

```rust
// Clone row data once, cache it
rows.push(dbt_data[row_off..row_off + row_bytes].to_vec());
```

### 5. Inline Accessors

```rust
#[inline]
fn read_u32_le(data: &[u8], offset: usize) -> u32 {
    if offset + 4 > data.len() { return 0; }
    u32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]])
}
```

### 6. Cached Column Values

For frequently accessed fields, pre-compute:

```rust
// SPECIFIC_CAR accessors — cache column indices
impl SpecDBTable {
    pub fn car_price(&self, row: usize) -> u32 {
        if row >= self.rows.len() { return 0; }
        let data = &self.rows[row];
        if data.len() >= 8 { read_u32_le(data, 4) } else { 0 }
    }
    
    pub fn car_category(&self, row: usize) -> u8 {
        if row >= self.rows.len() { return 0; }
        if self.columns.len() > 8 {
            self.read_column_value(row, &self.columns[8]) as u8
        } else { 0 }
    }
}
```

### 7. Bit-Packed Fields

Handle variable-width columns:

```rust
pub fn read_column_value(&self, row: usize, col: &ColumnDef) -> u32 {
    if row >= self.rows.len() { return 0; }
    let data = &self.rows[row];
    let byte_off = col.offset as usize;
    let bits = col.col_type;
    
    match bits {
        32 => read_u32_le(data, byte_off),
        16 => read_u16_le(data, byte_off) as u32,
        8 => data[byte_off] as u32,
        n if n < 8 => {
            // Bit-packed: extract n bits
            let mut val: u32 = 0;
            for i in 0..n as u32 {
                let bit_pos = byte_off as u32 * 8 + i;
                let byte_idx = (bit_pos / 8) as usize;
                let bit_idx = bit_pos % 8;
                if byte_idx < data.len() && (data[byte_idx] >> bit_idx) & 1 != 0 {
                    val |= 1 << i;
                }
            }
            val
        }
        _ => 0,
    }
}
```

### 8. Rc<RefCell<SpecDB>> Pattern

Share SpecDB across native closures:

```rust
// Passed to register functions
let specdb = Rc::new(RefCell::new(SpecDB::new()));
pub fn register_gtengine(registry: &mut NativeRegistry, specdb: Rc<RefCell<SpecDB>>) {
    // Each closure clones the Rc
    let s = specdb.clone();
    registry.register("main,gtengine,MSpecDB,getCarPrice", Rc::new(move |args| {
        let sd = s.borrow();  // RefCell borrow
        // ... access tables
    }));
}
```

---

## Memory Layout

### Per Table

| Component | Size |
|----------|------|
| `rows` | row_count × stride |
| `string_block` | Variable |
| `columns` | col_count × 8 |
| `id_to_row` | idi_count × 12 (HashMap overhead) |

### Total Estimate

- 837 cars × ~100 bytes = ~84 KB
- 85 tracks × ~200 bytes = ~17 KB
- 6042 variations × ~32 bytes = ~193 KB
- **Total**: ~300 KB (loaded)

---

## Error Handling

```rust
pub fn from_files(...) -> Result<Self, String> {
    // 1. Check minimum size
    if dbt_data.len() < 24 { return Err(...); }
    
    // 2. Check magic
    if &dbt_data[0..4] != b"GTDB" { return Err(...); }
    
    // 3. Clamp row count to available data
    let max_rows = (file_size.saturating_sub(header_size)) / row_bytes.max(1);
    let actual = (row_count as usize).min(max_rows);
    
    // 4. Validate bounds on each read
    if byte_off + 4 > data.len() { return 0; }
}
```

---

## Loading Sequence

```
┌─────────────────────────────────────────────────────────��───────┐
│ specdb = Rc::new(RefCell::new(SpecDB::new()))               │
└─────────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────────┐
│ specdb.borrow_mut().set_base_path("specdb/GT_PSP_JP2817") │
└─────────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────────┐
│ For each .dbt file in directory:                           │
│     load_table(table_name) → parse .dbt + optional .idi     │
│         ↓                                                 │
│     rows: Vec<Vec<u8>> (cloned row data)                   │
│     columns: Vec<ColumnDef> (descriptors)                │
│     id_to_row: HashMap (from .idi)                       │
│     string_block: Vec<u8> (UTF-16LE)                    │
└─────────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────────┐
│ Register native accessors with SpecDB Rc:               │
│     main,gtengine,MSpecDB,getCarPrice → rows[idx][off]  │
│     main,gtengine,MSpecDB,getCarName → string_block     │
│     main,gtengine,MSpecDB,getEngineSpec → relation    │
└─────────────────────────────────────────────────────────────────┘
```

---

## Current Status

| Feature | Status |
|---------|--------|
| .dbt loading | ✅ |
| .idi index loading | ✅ |
| Column descriptors | ✅ |
| Bit-packed fields | ✅ |
| UTF-16LE strings | ✅ |
| Row cloning | ✅ |
| ID → row lookup | ✅ |
| Pre-allocation | ✅ |
| Bit-packed support | ✅ |
| Cached column accessors | ✅ |

---

## See Also

- [[10_PC_Port/04_SpecDB_Reader|SpecDB Reader]]
- [[10_PC_Port/06_Native_API|Native API]]
- [[10_PC_Port/08_VM_Initialization|VM Initialization]]