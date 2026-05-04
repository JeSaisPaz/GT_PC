use std::cell::RefCell;
use std::rc::Rc;
use crate::engine::specdb::SpecDB;
use crate::engine::assets_root;
use crate::vm::value::*;

#[inline]
fn read_u32_le(data: &[u8], offset: usize) -> u32 {
    if offset + 4 > data.len() { return 0; }
    u32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]])
}

fn car_to_engine(sd: &SpecDB, car_idx: usize) -> Option<usize> {
    let car = sd.get_table("GENERIC_CAR")?;
    if car_idx >= car.row_count() { return None; }
    let dp_id = car.car_default_parts(car_idx);
    let dp = sd.get_table("DEFAULT_PARTS")?;
    let dp_row = dp.get_row_by_id(dp_id)?;
    let dpd = &dp.rows[dp_row];
    // DEFAULT_PARTS.Engine: column[11] for PSP (offset varies by .dbt descriptors)
    if dp.columns.len() <= 11 { return None; }
    let eng_id = dp.read_column_value(dp_row, &dp.columns[11]);
    let eng = sd.get_table("ENGINE")?;
    eng.get_row_by_id(eng_id)
}

fn car_to_chassis(sd: &SpecDB, car_idx: usize) -> Option<usize> {
    let car = sd.get_table("GENERIC_CAR")?;
    if car_idx >= car.row_count() { return None; }
    let dp_id = car.car_default_parts(car_idx);
    let dp = sd.get_table("DEFAULT_PARTS")?;
    let dp_row = dp.get_row_by_id(dp_id)?;
    // DEFAULT_PARTS.Chassis: column[4] for PSP
    if dp.columns.len() <= 4 { return None; }
    let ch_id = dp.read_column_value(dp_row, &dp.columns[4]);
    let ch = sd.get_table("CHASSIS")?;
    ch.get_row_by_id(ch_id)
}

fn car_to_drivetrain(sd: &SpecDB, car_idx: usize) -> Option<usize> {
    let car = sd.get_table("GENERIC_CAR")?;
    if car_idx >= car.row_count() { return None; }
    let dp_id = car.car_default_parts(car_idx);
    let dp = sd.get_table("DEFAULT_PARTS")?;
    let dp_row = dp.get_row_by_id(dp_id)?;
    // DEFAULT_PARTS.DriveTrain: column[9] for PSP
    if dp.columns.len() <= 9 { return None; }
    let dt_id = dp.read_column_value(dp_row, &dp.columns[9]);
    let dt = sd.get_table("DRIVETRAIN")?;
    dt.get_row_by_id(dt_id)
}

fn car_to_displacement(sd: &SpecDB, car_idx: usize) -> Option<usize> {
    let car = sd.get_table("GENERIC_CAR")?;
    if car_idx >= car.row_count() { return None; }
    let dp_id = car.car_default_parts(car_idx);
    let dp = sd.get_table("DEFAULT_PARTS")?;
    let dp_row = dp.get_row_by_id(dp_id)?;
    // DEFAULT_PARTS.Displacement: column[14] for PSP
    if dp.columns.len() <= 14 { return None; }
    let ds_id = dp.read_column_value(dp_row, &dp.columns[14]);
    let ds = sd.get_table("DISPLACEMENT")?;
    ds.get_row_by_id(ds_id)
}

/// Register all gtengine native API stubs.
pub fn register_gtengine(registry: &mut crate::vm::native::NativeRegistry, specdb: Rc<RefCell<SpecDB>>) {
    let s = specdb.clone();
    registry.register("main,gtengine,MSpecDB,initialize", Rc::new(move |_| {
        let mut sd = s.borrow_mut();
        let path = format!("{}\\specdb\\GT_PSP_JP2817", assets_root());
        if let Err(e) = sd.load_all(&path) { eprintln!("SpecDB error: {}", e); return Value::Int(0); }
        eprintln!("SpecDB loaded {} tables", sd.tables.len());
        Value::Int(1)
    }));
    registry.register("main,gtengine,MSpecDB,finalize", Rc::new(|_| Value::Void));

    // ─── GENERIC_CAR field accessors ──────────────────────────
    for (name, offset, bits) in &[
        ("Price", 8u32, 32u16), ("PowerMax", 14, 16), ("PowerMin", 16, 16),
        ("Year", 12, 16), ("Category", 21, 8), ("Country", 18, 8),
    ] {
        let path = format!("main,gtengine,MSpecDB,getCar{}", name);
        let sd = specdb.clone();
        let col_off = *offset as usize;
        registry.register(&path, Rc::new(move |args: &[Value]| {
            let idx = args.first().and_then(|a| a.as_i32()).unwrap_or(0) as usize;
            let sd = sd.borrow();
            if let Some(t) = sd.get_table("GENERIC_CAR") {
                if idx < t.row_count() {
                    let d = &t.rows[idx];
                    let v = if *bits == 32 && d.len() >= col_off + 4 { read_u32_le(d, col_off) as i32 }
                             else if *bits == 16 && d.len() >= col_off + 2 { u16::from_le_bytes([d[col_off], d[col_off+1]]) as i32 }
                             else if *bits == 8 && d.len() > col_off { d[col_off] as i32 }
                             else { 0 };
                    return Value::Int(v);
                }
            }
            Value::Int(0)
        }));
    }

    // ─── Car label list ──────────────────────────────────────
    let s4 = specdb.clone();
    registry.register("main,gtengine,MSpecDB,getCarLabelList", Rc::new(move |_| {
        let sd = s4.borrow();
        let mut labels = Vec::new();
        if let Some(t) = sd.get_table("GENERIC_CAR") {
            for row in 0..t.row_count().min(837) {
                labels.push(Value::String(Rc::new(format!("car_{}", row))));
            }
        }
        Value::Array(Rc::new(labels))
    }));

    // ─── Car name from CAR_NAME_american ─────────────────────
    let s5 = specdb.clone();
    registry.register("main,gtengine,MSpecDB,getCarName", Rc::new(move |args: &[Value]| {
        let idx = args.first().and_then(|a| a.as_i32()).unwrap_or(0) as usize;
        let sd = s5.borrow();
        if let Some(n) = sd.get_table("CAR_NAME_american") {
            if idx < n.row_count() {
                let name = n.car_name_by_row(idx);
                if !name.is_empty() { return Value::String(Rc::new(name)); }
            }
        }
        Value::String(Rc::new(format!("Car_{}", idx)))
    }));

    // ─── Car code (search by name) ──────────────────────────
    let s6 = specdb.clone();
    registry.register("main,gtengine,MSpecDB,getCarCode", Rc::new(move |args| {
        let _name = args.first().map(|a| a.to_string()).unwrap_or_default();
        let sd = s6.borrow();
        sd.get_table("GENERIC_CAR").map(|_| Value::UInt(1)).unwrap_or(Value::UInt(0))
    }));

    // ─── Car label ──────────────────────────────────────────
    let s_lab = specdb.clone();
    registry.register("main,gtengine,MSpecDB,getCarLabel", Rc::new(move |args| {
        let idx = args.first().and_then(|a| a.as_i32()).unwrap_or(0) as usize;
        let sd = s_lab.borrow();
        sd.get_table("GENERIC_CAR").map(|t| {
            if idx < t.row_count() { Value::String(Rc::new(format!("CAR{:03}", idx))) }
            else { Value::String(Rc::new("CAR000".to_string())) }
        }).unwrap_or(Value::String(Rc::new("CAR000".to_string())))
    }));
    let s_short = specdb.clone();
    registry.register("main,gtengine,MSpecDB,getCarShortName", Rc::new(move |args| {
        let idx = args.first().and_then(|a| a.as_i32()).unwrap_or(0) as usize;
        let sd = s_short.borrow();
        sd.get_table("GENERIC_CAR").map(|t| {
            Value::String(Rc::new(format!("C{:03}", (idx % t.row_count()) + 1)))
        }).unwrap_or(Value::String(Rc::new("C000".to_string())))
    }));

    // ─── Car category (from GENERIC_CAR) ────────────────────
    let s_cat = specdb.clone();
    registry.register("main,gtengine,MSpecDB,getCarCategory", Rc::new(move |args| {
        let idx = args.first().and_then(|a| a.as_i32()).unwrap_or(0) as usize;
        let sd = s_cat.borrow();
        if let Some(t) = sd.get_table("GENERIC_CAR") {
            if idx < t.row_count() { return Value::Int(t.car_category(idx) as i32); }
        }
        Value::Int(0)
    }));

    // ─── Drive train ────────────────────────────────────────
    let s_dt = specdb.clone();
    registry.register("main,gtengine,MSpecDB,getCarDriveTrain", Rc::new(move |args| {
        let idx = args.first().and_then(|a| a.as_i32()).unwrap_or(0) as usize;
        let sd = s_dt.borrow();
        if let Some(dt) = car_to_drivetrain(&sd, idx) {
            if let Some(t) = sd.get_table("DRIVETRAIN") {
                return Value::Int(t.drivetrain_type(dt) as i32);
            }
        }
        Value::Int(0)
    }));

    // ─── Car variation (color count) ────────────────────────
    let s_var = specdb.clone();
    registry.register("main,gtengine,MSpecDB,getCarVariation", Rc::new(move |args| {
        let idx = args.first().and_then(|a| a.as_i32()).unwrap_or(0) as usize;
        let sd = s_var.borrow();
        let count = sd.get_table("VARIATION").map(|t| t.row_count()).unwrap_or(1);
        Value::Int((count % 10 + 1) as i32) // return 1-10 variations
    }));

    // ─── Catalog data (power, torque, mass, dimensions, displacement) ─
    let s_ps = specdb.clone();
    registry.register("main,gtengine,MSpecDB,getCarCatalogPs", Rc::new(move |args| {
        let idx = args.first().and_then(|a| a.as_i32()).unwrap_or(0) as usize;
        let sd = s_ps.borrow();
        sd.get_table("GENERIC_CAR").map(|t| {
            if idx < t.row_count() { Value::Int(t.car_power_max(idx) as i32) }
            else { Value::Int(200) }
        }).unwrap_or(Value::Int(200))
    }));

    let s_tq = specdb.clone();
    registry.register("main,gtengine,MSpecDB,getCarCatalogTq", Rc::new(move |args| {
        let idx = args.first().and_then(|a| a.as_i32()).unwrap_or(0) as usize;
        let sd = s_tq.borrow();
        if let Some(eng_row) = car_to_engine(&sd, idx) {
            if let Some(t) = sd.get_table("ENGINE") {
                return Value::Int(t.engine_torque_value(eng_row) as i32);
            }
        }
        Value::Int(200)
    }));

    let s_ms = specdb.clone();
    registry.register("main,gtengine,MSpecDB,getCarCatalogMs", Rc::new(move |args| {
        let idx = args.first().and_then(|a| a.as_i32()).unwrap_or(0) as usize;
        let sd = s_ms.borrow();
        if let Some(ch_row) = car_to_chassis(&sd, idx) {
            if let Some(t) = sd.get_table("CHASSIS") {
                return Value::Int(t.chassis_mass(ch_row) as i32);
            }
        }
        Value::Int(1200)
    }));

    let s_len = specdb.clone();
    registry.register("main,gtengine,MSpecDB,getCarCatalogLength", Rc::new(move |args| {
        let idx = args.first().and_then(|a| a.as_i32()).unwrap_or(0) as usize;
        let sd = s_len.borrow();
        if let Some(ch_row) = car_to_chassis(&sd, idx) {
            if let Some(t) = sd.get_table("CHASSIS") {
                return Value::Int(t.chassis_length(ch_row) as i32);
            }
        }
        Value::Int(4600)
    }));

    let s_wid = specdb.clone();
    registry.register("main,gtengine,MSpecDB,getCarCatalogWidth", Rc::new(move |args| {
        let idx = args.first().and_then(|a| a.as_i32()).unwrap_or(0) as usize;
        let sd = s_wid.borrow();
        if let Some(ch_row) = car_to_chassis(&sd, idx) {
            if let Some(t) = sd.get_table("CHASSIS") {
                return Value::Int(t.chassis_width(ch_row) as i32);
            }
        }
        Value::Int(1745)
    }));

    let s_hgt = specdb.clone();
    registry.register("main,gtengine,MSpecDB,getCarCatalogHeight", Rc::new(move |args| {
        let idx = args.first().and_then(|a| a.as_i32()).unwrap_or(0) as usize;
        let sd = s_hgt.borrow();
        if let Some(ch_row) = car_to_chassis(&sd, idx) {
            if let Some(t) = sd.get_table("CHASSIS") {
                return Value::Int(t.chassis_height(ch_row) as i32);
            }
        }
        Value::Int(1200)
    }));

    let s_disp = specdb.clone();
    registry.register("main,gtengine,MSpecDB,getCarCatalogDisplacement", Rc::new(move |args| {
        let idx = args.first().and_then(|a| a.as_i32()).unwrap_or(0) as usize;
        let sd = s_disp.borrow();
        if let Some(ds_row) = car_to_displacement(&sd, idx) {
            if let Some(t) = sd.get_table("DISPLACEMENT") {
                return Value::Int(t.displacement_cc(ds_row) as i32);
            }
        }
        Value::Int(2000)
    }));

    let s_psrpm = specdb.clone();
    registry.register("main,gtengine,MSpecDB,getCarCatalogPsRpm", Rc::new(move |args| {
        let idx = args.first().and_then(|a| a.as_i32()).unwrap_or(0) as usize;
        let sd = s_psrpm.borrow();
        if let Some(eng_row) = car_to_engine(&sd, idx) {
            // ENGINE.psrpm string at offset 0x14 — display only. Use revlimit (offset 0x55) × 100
            if let Some(t) = sd.get_table("ENGINE") {
                let d = &t.rows[eng_row];
                if d.len() > 0x55 { return Value::Int((d[0x55] as i32) * 100); }
            }
        }
        Value::Int(6500)
    }));

    let s_tqrpm = specdb.clone();
    registry.register("main,gtengine,MSpecDB,getCarCatalogTqRpm", Rc::new(move |args| {
        let idx = args.first().and_then(|a| a.as_i32()).unwrap_or(0) as usize;
        let sd = s_tqrpm.borrow();
        if let Some(eng_row) = car_to_engine(&sd, idx) {
            // ENGINE.torquerpm string at offset 0x18 — display only. Estimate at 70% of revlimit
            if let Some(t) = sd.get_table("ENGINE") {
                let d = &t.rows[eng_row];
                if d.len() > 0x55 { return Value::Int((d[0x55] as i32 * 70 / 100) as i32); }
            }
        }
        Value::Int(4500)
    }));

    // ─── Power / Weight (generic) ───────────────────────────
    let s_pwr = specdb.clone();
    registry.register("main,gtengine,MSpecDB,getPower", Rc::new(move |args| {
        let idx = args.first().and_then(|a| a.as_i32()).unwrap_or(0) as usize;
        let sd = s_pwr.borrow();
        sd.get_table("GENERIC_CAR").map(|t| {
            if idx < t.row_count() { Value::Int(t.car_power_max(idx) as i32) }
            else { Value::Int(200) }
        }).unwrap_or(Value::Int(200))
    }));
    let s_wgt = specdb.clone();
    registry.register("main,gtengine,MSpecDB,getWeight", Rc::new(move |args| {
        let idx = args.first().and_then(|a| a.as_i32()).unwrap_or(0) as usize;
        let sd = s_wgt.borrow();
        if let Some(ch_row) = car_to_chassis(&sd, idx) {
            if let Some(t) = sd.get_table("CHASSIS") {
                return Value::Int(t.chassis_mass(ch_row) as i32);
            }
        }
        Value::Int(1300)
    }));

    // ─── Course accessors ────────────────────────────────────
    let s_cname = specdb.clone();
    registry.register("main,gtengine,MSpecDB,getCourseName", Rc::new(move |args| {
        let idx = args.first().and_then(|a| a.as_i32()).unwrap_or(0) as usize;
        let sd = s_cname.borrow();
        if let Some(t) = sd.get_table("COURSE") {
            if idx < t.row_count() {
                let name = t.course_name_eng(idx);
                if !name.is_empty() { return Value::String(Rc::new(name)); }
            }
        }
        Value::String(Rc::new(format!("Course_{}", idx)))
    }));

    let s_ccode = specdb.clone();
    registry.register("main,gtengine,MSpecDB,getCourseCode", Rc::new(move |args| {
        let _name = args.first().map(|a| a.to_string()).unwrap_or_default();
        let sd = s_ccode.borrow();
        sd.get_table("COURSE").map(|_| Value::UInt(1)).unwrap_or(Value::UInt(0))
    }));

    registry.register("main,gtengine,MSpecDB,getCourseCondition", Rc::new(|_| Value::Int(0)));
    registry.register("main,gtengine,MSpecDB,getCourseInformation", Rc::new(|_| {
        Value::Object(Rc::new(ObjectInstance { class_path: "MCourseInfo".to_string(), fields: vec![] }))
    }));
    registry.register("main,gtengine,MSpecDB,getCourseDataPath", Rc::new(|args| {
        let idx = args.first().and_then(|a| a.as_i32()).unwrap_or(0);
        Value::String(Rc::new(format!("crs/c{:03}", idx + 1)))
    }));

    // ─── Race accessors ──────────────────────────────────────
    let s_rlab = specdb.clone();
    registry.register("main,gtengine,MSpecDB,getRaceLabel", Rc::new(move |args| {
        let idx = args.first().and_then(|a| a.as_i32()).unwrap_or(0) as usize;
        let sd = s_rlab.borrow();
        if let Some(t) = sd.get_table("RACE") {
            if idx < t.row_count() {
                let label = t.race_course_label(idx);
                if !label.is_empty() { return Value::String(Rc::new(label)); }
            }
        }
        Value::String(Rc::new(format!("Race_{}", idx)))
    }));
    registry.register("main,gtengine,MSpecDB,getRaceCodeFromCar", Rc::new(|_| Value::UInt(0)));

    // ─── Opponent cars from ENEMY_CARS ──────────────────────
    let s_opp = specdb.clone();
    registry.register("main,gtengine,MSpecDB,getOpponentCars", Rc::new(move |_| {
        let sd = s_opp.borrow();
        if let Some(t) = sd.get_table("ENEMY_CARS") {
            let mut cars = Vec::new();
            // ENEMY_CARS likely has car code at offset 0 (Int)
            // Return first 12 as sample opponents
            for row in 0..t.row_count().min(12) {
                let d = &t.rows[row];
                if d.len() >= 4 {
                    cars.push(Value::UInt(read_u32_le(d, 0)));
                }
            }
            return Value::Array(Rc::new(cars));
        }
        Value::Array(Rc::new(vec![]))
    }));

    registry.register("main,gtengine,MSpecDB,isDirt", Rc::new(|_| Value::Bool(false)));
    registry.register("main,gtengine,MSpecDB,loadMenuInfo", Rc::new(|_| Value::Int(1)));
    registry.register("main,gtengine,MSpecDB,unloadMenuInfo", Rc::new(|_| Value::Void));
    registry.register("main,gtengine,MSpecDB,NO_CODE64", Rc::new(|_| Value::UInt(0xFFFFFFFF)));

    // ─── Car info / Engine info (return objects) ────────────
    registry.register("main,gtengine,MSpecDB,getCarInfo", Rc::new(|_| {
        Value::Object(Rc::new(ObjectInstance { class_path: "MCarInfo".to_string(), fields: vec![] }))
    }));
    registry.register("main,gtengine,MSpecDB,getEngineSpec", Rc::new(|_| {
        Value::Object(Rc::new(ObjectInstance { class_path: "MEngineSpec".to_string(), fields: vec![] }))
    }));

    // ─── Race parameter ─────────────────────────────────────
    registry.register("main,gtengine,MSpecDB,getRaceParameter", Rc::new(|args| {
        let _idx = args.first().and_then(|a| a.as_i32()).unwrap_or(0) as usize;
        Value::Object(Rc::new(ObjectInstance { class_path: "MRaceParameter".to_string(), fields: vec![] }))
    }));

    // ─── Constructors ───────────────────────────────────────
    for cls in &["MRaceParameter", "MCarParameter", "MCarDriverParameter", "MCamera",
                  "MOrganizer", "MRaceOperator", "MRaceSound"] {
        let path = format!("main,gtengine,{}", cls);
        let cn = cls.to_string();
        registry.register(&path, Rc::new(move |_| {
            Value::Object(Rc::new(ObjectInstance { class_path: cn.clone(), fields: vec![] }))
        }));
    }

    // ─── MOrganizer methods ──────────────────────────────────
    registry.register("main,gtengine,MOrganizer,initialize", Rc::new(|_| {
        eprintln!("[MOrganizer] initialize");
        Value::Void
    }));
    registry.register("main,gtengine,MOrganizer,startGameObjectLoop", Rc::new(|_| {
        eprintln!("[MOrganizer] startGameObjectLoop");
        Value::Void
    }));

    // ─── MRaceOperator methods ───────────────────────────────
    registry.register("main,gtengine,MRaceOperator,setOrganizer", Rc::new(|args: &[Value]| {
        eprintln!("[MRaceOperator] setOrganizer");
        Value::Void
    }));
    registry.register("main,gtengine,MRaceOperator,getMemoryAssign_Standard", Rc::new(|_| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MMemoryAssign".to_string(),
            fields: vec![],
        }))
    }));
    registry.register("main,gtengine,MRaceOperator,getMemoryAssign_WithGhost", Rc::new(|_| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MMemoryAssign".to_string(),
            fields: vec![],
        }))
    }));

    registry.register("main,gtengine,MCarParameter,loadModel", Rc::new(|args| {
        let car_id = args.first().map(|v| match v { Value::Int(i) => *i as u32, _ => 0 }).unwrap_or(0);
        eprintln!("[MCar] loadModel car_id={}", car_id);
        match crate::engine::pdiapp::load_car_model(car_id) {
            Ok(m) => { eprintln!("[MCar] {}v {}tri", m.vertices.len(), m.triangles.len()); Value::Bool(true) }
            Err(e) => { eprintln!("[MCar] {}", e); Value::Bool(false) }
        }
    }));

    registry.register("main,gtengine,MCamera,load", Rc::new(|args| {
        let course_id = args.first().map(|v| match v { Value::Int(i) => *i as u32, _ => 0 }).unwrap_or(0);
        eprintln!("[MCam] load course_id={}", course_id);
        match crate::engine::pdiapp::load_camera(course_id) {
            Ok(cam) => { eprintln!("[MCam] ({:.1},{:.1},{:.1}) fov={}", cam.position.0, cam.position.1, cam.position.2, cam.fov); Value::Bool(true) }
            Err(e) => { eprintln!("[MCam] {}", e); Value::Bool(false) }
        }
    }));

    registry.register("main,gtengine,MRaceSound,initializeGlobalParameter", Rc::new(|_| Value::Int(0)));
    registry.register("main,gtengine,EnemySetUtil,beginWithXml", Rc::new(|_| Value::Bool(true)));
    registry.register("main,gtengine,EnemySetUtil,end", Rc::new(|_| Value::Void));

    // ─── Table row count getters ────────────────────────────
    for table_name in &["GENERIC_CAR","ENGINE","SUSPENSION","CHASSIS","DRIVETRAIN",
                         "GEAR","BRAKE","FRONTTIRE","REARTIRE","LSD","COURSE","RACE",
                         "VARIATION","MAKER","TUNER","ENEMY_CARS"] {
        let path = format!("main,gtengine,MSpecDB,get{}", table_name);
        let tn = table_name.to_string();
        let sd = specdb.clone();
        registry.register(&path, Rc::new(move |_| {
            sd.borrow().get_table(&tn).map(|t| Value::Int(t.row_count() as i32)).unwrap_or(Value::Int(0))
        }));
    }

    // ─── Enums ──────────────────────────────────────────────
    registry.register("main,gtengine,CourseCondition,DIRT", Rc::new(|_| Value::Int(1)));
    registry.register("main,gtengine,CourseCondition,SNOW", Rc::new(|_| Value::Int(2)));
    registry.register("main,gtengine,CourseCondition,WET", Rc::new(|_| Value::Int(1)));
    registry.register("main,pdiext,OUTPUTMONAURALMODE,MONAURAL_NORMAL", Rc::new(|_| Value::Int(0)));
    registry.register("main,pdiext,OUTPUTSTEREOMODE,STEREO_NORMAL", Rc::new(|_| Value::Int(0)));
    registry.register("main,pdiext,OUTPUTSTEREOMODE,STEREO_DOLBY_PL2", Rc::new(|_| Value::Int(1)));
    registry.register("main,pdiext,VoucherResultCode,ResultCodeOK", Rc::new(|_| Value::Int(0)));
    registry.register("main,pdiext,VoucherResultCode,ResultCodeUnknown", Rc::new(|_| Value::Int(1)));
    registry.register("main,pdiext,VoucherResultCode,ResultCodeDRMTimeLimit", Rc::new(|_| Value::Int(-1)));

    eprintln!("Registered additional gtengine/native API stubs");
}
