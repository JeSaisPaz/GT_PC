/// GT PSP SpecDB binary format reader.
///
/// Based on Nenkai's GTSpecDB and GT-File-Specifications-Documentation.
///
/// .dbt format (PSP variant):
///   Offset  Size  Field
///   0x00    4     Magic: "GTDB"
///   0x04    4     Version + flags (u32 LE, 0x00010008 for PSP)
///   0x08    4     Row count
///   0x0C    4     Column count
///   0x10    4     Row stride (bytes per row)
///   0x14    4     Padding (0)
///   0x18    N*8   Column descriptors
///   ...           Row data (row_count × stride bytes)
///   ...           String data (4-byte length prefix, then UTF-16LE)
///
/// Column descriptor (8 bytes):
///   u16 field_offset
///   u16 field_bitsize  (0=dynamic, 2/4/8/16/32 = fixed width)
///   u32 reserved (usually 0)
///
/// .idi format:
///   Offset  Size  Field
///   0x00    4     Magic: "GTID"
///   0x04    4     Key count (= row count in DBT)
///   0x08    4     Padding (0)
///   0x0C    4     Table ID
///   0x10    N*8   Keys: (string_offset: u32, dbt_row_id: u32)
///   ...           String table

use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ColumnDef {
    pub offset: u16,
    pub col_type: u16,
}

#[derive(Debug, Clone)]
pub struct SpecDBTable {
    pub name: String,
    pub columns: Vec<ColumnDef>,
    pub stride: u32,
    pub rows: Vec<Vec<u8>>,
    pub id_to_row: HashMap<u32, usize>,
    pub string_block: Vec<u8>,
    pub string_offset: u32,
    pub row_count: u32,
    pub col_count: u32,
}

impl SpecDBTable {
    pub fn from_files(dbt_data: &[u8], idi_data: Option<&[u8]>, table_name: &str) -> Result<Self, String> {
        if dbt_data.len() < 24 {
            return Err("DBT file too short".to_string());
        }
        if &dbt_data[0..4] != b"GTDB" {
            return Err(format!("Bad DBT magic: {:02X?}", &dbt_data[0..4]));
        }

        let row_count = read_u32_le(dbt_data, 0x08);
        let col_count = read_u32_le(dbt_data, 0x0C);
        let stride = read_u32_le(dbt_data, 0x10);
        let file_size = dbt_data.len() as u32;

        // Clamp column count to available descriptor data
        let desc_bytes = file_size.saturating_sub(0x18);
        let max_cols_from_file = desc_bytes / 8;
        let actual_cols = col_count.min(max_cols_from_file);

        // Column descriptors at offset 0x18
        let col_desc_start = 0x18u32;
        let mut columns = Vec::with_capacity(actual_cols as usize);
        for i in 0..actual_cols {
            let pos = (col_desc_start + i * 8) as usize;
            if pos + 8 > dbt_data.len() {
                break;
            }
            let col_off = read_u16_le(dbt_data, pos);
            let col_type = read_u16_le(dbt_data, pos + 2);
            columns.push(ColumnDef { offset: col_off, col_type });
        }

        let col_desc_end = col_desc_start + actual_cols * 8;

        // Row data area
        let row_data_start = col_desc_end;
        let mut rows: Vec<Vec<u8>> = Vec::new();

        if stride > 0 && row_count > 0 {
            let row_bytes = stride as usize;
            // Validate row count against available data to prevent OOM
            let max_rows = (dbt_data.len().saturating_sub(row_data_start as usize)) / row_bytes.max(1);
            let actual_row_count = (row_count as usize).min(max_rows);
            for i in 0..actual_row_count {
                let row_off = (row_data_start + i as u32 * row_bytes as u32) as usize;
                if row_off + row_bytes > dbt_data.len() {
                    break;
                }
                rows.push(dbt_data[row_off..row_off + row_bytes].to_vec());
            }
        }

        // String block starts after row data
        let str_start = if stride > 0 {
            (row_data_start + row_count * stride) as usize
        } else {
            col_desc_end as usize
        };
        let string_block = if str_start < dbt_data.len() {
            dbt_data[str_start..].to_vec()
        } else {
            Vec::new()
        };

        // Parse IDI
        let mut id_to_row = HashMap::new();
        if let Some(idi) = idi_data {
            if idi.len() >= 16 && &idi[0..4] == b"GTID" {
                let idi_count = read_u32_le(idi, 0x04) as usize;
                for i in 0..idi_count {
                    let entry_pos = 0x10 + i * 8;
                    if entry_pos + 8 > idi.len() { break; }
                    let row_id = read_u32_le(idi, entry_pos + 4);
                    id_to_row.insert(row_id, i);
                }
            }
        }

        Ok(SpecDBTable {
            name: table_name.to_string(),
            columns,
            stride,
            rows,
            id_to_row,
            string_block,
            string_offset: str_start as u32,
            row_count,
            col_count,
        })
    }

    pub fn read_u32(&self, row: usize, col: &ColumnDef) -> u32 {
        self.read_column_value(row, col)
    }

    pub fn read_i32(&self, row: usize, col: &ColumnDef) -> i32 {
        self.read_u32(row, col) as i32
    }

    pub fn read_u16(&self, row: usize, col: &ColumnDef) -> u16 {
        self.read_u32(row, col) as u16
    }

    // ─── GTSpecDB Field-Specific Readers (GTPSP schema) ─────

    /// GENERIC_CAR.DefaultParts (Key, offset 0, 4 bytes for GTPSP): row ID in DEFAULT_PARTS
    pub fn car_default_parts(&self, row: usize) -> u32 {
        if row >= self.rows.len() { return 0; }
        let data = &self.rows[row];
        if data.len() >= 4 { read_u32_le(data, 0) } else { 0 }
    }

    /// GENERIC_CAR.Price (Int, offset 4, 4 bytes)
    pub fn car_price(&self, row: usize) -> u32 {
        if row >= self.rows.len() { return 0; }
        let data = &self.rows[row];
        if data.len() >= 8 { read_u32_le(data, 4) } else { 0 }
    }

    /// GENERIC_CAR.Year (Short, offset 8?, 2 bytes) 
    /// Note: offsets are from .dbt column descriptors, may differ from GTSpecDB schema
    pub fn car_year(&self, row: usize) -> u16 {
        if row >= self.rows.len() { return 0; }
        let data = &self.rows[row];
        if data.len() >= 12 && self.columns.len() > 2 {
            self.read_column_value(row, &self.columns[2]) as u16
        } else { 0 }
    }

    /// GENERIC_CAR.PowerMax (from column[3])
    pub fn car_power_max(&self, row: usize) -> u32 {
        if row >= self.rows.len() { return 0; }
        if self.columns.len() > 3 {
            self.read_column_value(row, &self.columns[3])
        } else { 0 }
    }

    /// GENERIC_CAR.PowerMin (from column[4])
    pub fn car_power_min(&self, row: usize) -> u32 {
        if row >= self.rows.len() { return 0; }
        if self.columns.len() > 4 {
            self.read_column_value(row, &self.columns[4])
        } else { 0 }
    }

    /// GENERIC_CAR.Maker (from column[6])
    pub fn car_maker(&self, row: usize) -> u8 {
        if row >= self.rows.len() { return 0; }
        if self.columns.len() > 6 {
            self.read_column_value(row, &self.columns[6]) as u8
        } else { 0 }
    }

    /// GENERIC_CAR.Country (from column[5])
    pub fn car_country(&self, row: usize) -> u8 {
        if row >= self.rows.len() { return 0; }
        if self.columns.len() > 5 {
            self.read_column_value(row, &self.columns[5]) as u8
        } else { 0 }
    }

    /// GENERIC_CAR.Category (from column[8])
    pub fn car_category(&self, row: usize) -> u8 {
        if row >= self.rows.len() { return 0; }
        if self.columns.len() > 8 {
            self.read_column_value(row, &self.columns[8]) as u8
        } else { 0 }
    }

    /// ENGINE.psvalue (column[8] for PSP): peak power PS
    pub fn engine_ps_value(&self, row: usize) -> u32 {
        if row >= self.rows.len() { return 0; }
        if self.columns.len() > 8 { self.read_column_value(row, &self.columns[8]) } else { 0 }
    }

    /// ENGINE.torquevalue (column[9] for PSP): peak torque kgf·m
    pub fn engine_torque_value(&self, row: usize) -> u32 {
        if row >= self.rows.len() { return 0; }
        if self.columns.len() > 9 { self.read_column_value(row, &self.columns[9]) } else { 0 }
    }

    /// ENGINE.soundNum (column[0] for PSP): sound bank ID
    pub fn engine_sound_id(&self, row: usize) -> u32 {
        if row >= self.rows.len() { return 0; }
        if !self.columns.is_empty() { self.read_column_value(row, &self.columns[0]) } else { 0 }
    }

    /// COURSE.NameEng (String pointer, column[2]): offset into string block
    pub fn course_name_eng(&self, row: usize) -> String {
        if row >= self.rows.len() || self.columns.len() < 3 { return String::new(); }
        let str_off = self.read_column_value(row, &self.columns[2]);
        self.read_string(str_off)
    }

    /// COURSE.NameJpn (String pointer, column[1])
    pub fn course_name_jpn(&self, row: usize) -> String {
        if row >= self.rows.len() || self.columns.len() < 2 { return String::new(); }
        let str_off = self.read_column_value(row, &self.columns[1]);
        self.read_string(str_off)
    }

    // ─── CHASSIS (from GTSpecDB) ──────────────────────────────
    /// CHASSIS.Length (column[0]): in mm
    pub fn chassis_length(&self, row: usize) -> u32 {
        if row >= self.rows.len() || self.columns.is_empty() { return 0; }
        self.read_column_value(row, &self.columns[0])
    }
    /// CHASSIS.Width (column[1])
    pub fn chassis_width(&self, row: usize) -> u32 {
        if row >= self.rows.len() || self.columns.len() < 2 { return 0; }
        self.read_column_value(row, &self.columns[1])
    }
    /// CHASSIS.Height (column[2])
    pub fn chassis_height(&self, row: usize) -> u32 {
        if row >= self.rows.len() || self.columns.len() < 3 { return 0; }
        self.read_column_value(row, &self.columns[2])
    }
    /// CHASSIS.Mass (column[3]) — in kg
    pub fn chassis_mass(&self, row: usize) -> u32 {
        if row >= self.rows.len() || self.columns.len() < 4 { return 0; }
        self.read_column_value(row, &self.columns[3])
    }

    // ─── DRIVETRAIN ──────────────────────────────────────────
    /// DRIVETRAIN.Type (column[0]): 0=FF, 1=FR, 2=MR, 3=4WD
    pub fn drivetrain_type(&self, row: usize) -> u32 {
        if row >= self.rows.len() || self.columns.is_empty() { return 0; }
        self.read_column_value(row, &self.columns[0])
    }

    // ─── DISPLACEMENT ────────────────────────────────────────
    /// DISPLACEMENT.displacement (column[0]) — in cc
    pub fn displacement_cc(&self, row: usize) -> u32 {
        if row >= self.rows.len() || self.columns.is_empty() { return 0; }
        self.read_column_value(row, &self.columns[0])
    }

    // ─── CAR_NAME (localized string table) ───────────────────
    /// CAR_NAME_* tables: row ID = GENERIC_CAR row ID, first column = string ptr
    pub fn car_name_by_row(&self, row: usize) -> String {
        if row >= self.rows.len() || self.columns.is_empty() { return String::new(); }
        let str_off = self.read_column_value(row, &self.columns[0]);
        let name = self.read_string(str_off);
        if !name.is_empty() { return name; }
        let d = &self.rows[row];
        let raw: Vec<u8> = d.iter().take_while(|&&b| b != 0).copied().collect();
        if !raw.is_empty() { String::from_utf8_lossy(&raw).to_string() }
        else { String::new() }
    }

    // ─── RACE (from GTSpecDB GTPSP format) ───────────────────
    /// RACE.CourseLabel (String ptr, column[0])
    pub fn race_course_label(&self, row: usize) -> String {
        if row >= self.rows.len() || self.columns.is_empty() { return String::new(); }
        let str_off = self.read_column_value(row, &self.columns[0]);
        self.read_string(str_off)
    }
    /// RACE.Prize (Int, column[5]): 1st place prize credits
    pub fn race_prize_1st(&self, row: usize) -> u32 {
        if row >= self.rows.len() || self.columns.len() < 6 { return 0; }
        self.read_column_value(row, &self.columns[5])
    }

    /// Read a column value using its bit size (handles bit-packed fields properly)
    pub fn read_column_value(&self, row: usize, col: &ColumnDef) -> u32 {
        if row >= self.rows.len() { return 0; }
        let data = &self.rows[row];
        let byte_off = col.offset as usize;
        let bits = col.col_type;

        match bits {
            0 => 0,
            32 => {
                if byte_off + 4 <= data.len() { read_u32_le(data, byte_off) } else { 0 }
            }
            16 => {
                if byte_off + 2 <= data.len() { read_u16_le(data, byte_off) as u32 } else { 0 }
            }
            8 => {
                if byte_off < data.len() { data[byte_off] as u32 } else { 0 }
            }
            n if n < 8 => {
                // Bit-packed field: extract n bits starting at byte_off * 8
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

    pub fn read_string(&self, offset: u32) -> String {
        let off = offset as usize;
        let sb = &self.string_block;
        if off >= sb.len() || off + 4 > sb.len() {
            return String::new();
        }
        let byte_len = read_u32_le(sb, off) as usize;
        if off + 4 + byte_len > sb.len() {
            return String::new();
        }
        let u16_data: Vec<u16> = sb[off + 4..off + 4 + byte_len]
            .chunks(2)
            .filter(|c| c.len() == 2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&u16_data).trim_end_matches(char::from(0)).to_string()
    }

    pub fn get_row_by_id(&self, id: u32) -> Option<usize> {
        self.id_to_row.get(&id).copied()
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Print a summary of the table
    pub fn print_summary(&self) {
        println!("  {}: {} rows, {} cols, {} bytes/row, {} actual rows, {} str",
            self.name, self.row_count, self.col_count, self.stride, self.rows.len(), self.string_block.len());
        for (i, col) in self.columns.iter().enumerate() {
            let sample = if !self.rows.is_empty() { self.read_column_value(0, col) } else { 0 };
            let sample2 = if self.rows.len() > 1 { self.read_column_value(1, col) } else { 0 };
            println!("    col[{}]: off={} bits={}  row0={} row1={}", i, col.offset, col.col_type, sample, sample2);
        }
        if !self.columns.is_empty() && !self.rows.is_empty() {
            let first = &self.rows[0];
            let max_show = self.stride.min(64) as usize;
            let hex: Vec<String> = first[..max_show].iter().map(|b| format!("{:02X}", b)).collect();
            println!("    row[0] hex (first {}B): {}", max_show, hex.join(" "));
        }
    }
}

pub struct SpecDB {
    pub tables: HashMap<String, SpecDBTable>,
    pub base_path: String,
}

impl SpecDB {
    pub fn new() -> Self {
        SpecDB { tables: HashMap::new(), base_path: String::new() }
    }

    pub fn set_base_path(&mut self, path: &str) {
        self.base_path = path.to_string();
    }

    pub fn load_table(&mut self, table_name: &str) -> Result<(), String> {
        let dbt_path = format!("{}/{}.dbt", self.base_path, table_name);
        let idi_path = format!("{}/{}.idi", self.base_path, table_name);
        let dbt_data = std::fs::read(&dbt_path)
            .map_err(|e| format!("{}: {}", table_name, e))?;
        let idi_data = std::fs::read(&idi_path).ok();
        let table = SpecDBTable::from_files(&dbt_data, idi_data.as_deref(), table_name)?;
        self.tables.insert(table_name.to_string(), table);
        Ok(())
    }

    pub fn get_table(&self, name: &str) -> Option<&SpecDBTable> {
        self.tables.get(name)
    }

    pub fn load_all(&mut self, path: &str) -> Result<(), String> {
        self.base_path = path.to_string();
        let dir = Path::new(path);
        if !dir.is_dir() {
            return Err(format!("SpecDB path not found: {}", path));
        }
        let mut loaded = 0u32;
        let mut failed = 0u32;
        for entry in std::fs::read_dir(dir).map_err(|e| format!("read_dir: {}", e))? {
            let entry = entry.map_err(|e| format!("entry: {}", e))?;
            let fname = entry.file_name().to_string_lossy().to_string();
            if fname.ends_with(".dbt") {
                let name = fname.trim_end_matches(".dbt");
                match self.load_table(name) {
                    Ok(_) => { loaded += 1; }
                    Err(e) => {
                        eprintln!("Warning: Failed {}: {}", name, e);
                        failed += 1;
                    }
                }
            }
        }
        eprintln!("SpecDB: loaded {} tables, {} failed", loaded, failed);
        if self.tables.is_empty() {
            return Err("No tables loaded".to_string());
        }
        Ok(())
    }

    pub fn load_directory(path: &str) -> Result<SpecDB, String> {
        let mut sd = SpecDB::new();
        sd.load_all(path)?;
        Ok(sd)
    }
}

#[inline]
fn read_u32_le(data: &[u8], offset: usize) -> u32 {
    if offset + 4 > data.len() { return 0; }
    u32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]])
}

#[inline]
fn read_u16_le(data: &[u8], offset: usize) -> u16 {
    if offset + 2 > data.len() { return 0; }
    u16::from_le_bytes([data[offset], data[offset+1]])
}