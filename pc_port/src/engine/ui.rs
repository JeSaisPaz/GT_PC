use std::collections::HashMap;
use crate::engine::graphics::get_thread_renderer;
use crate::engine::assets_root;

use std::sync::Mutex;
static UI_MANAGER: std::sync::OnceLock<Mutex<UiManager>> = std::sync::OnceLock::new();

fn get_ui() -> &'static Mutex<UiManager> {
    UI_MANAGER.get_or_init(|| Mutex::new(UiManager::new()))
}

pub fn with_ui<F, R>(f: F) -> R where F: FnOnce(&mut UiManager) -> R {
    let mut guard = get_ui().lock().unwrap();
    f(&mut *guard)
}

#[derive(Clone, Copy)]
pub struct RGBA { pub r: u8, pub g: u8, pub b: u8, pub a: u8 }

#[derive(Clone)]
pub enum WidgetKind {
    RootWindow, ColorFace { color: Vec<RGBA>, is_divide_color: bool, y_divide: Vec<f32>, color_divide: Vec<Vec<RGBA>>, opacity: f32 },
    TextFace { text: String, font: String, color: RGBA, shadow: RGBA, shadow_value: i32, scale_x: f32, scale_y: f32, align: i32, multiline: bool },
    ImageFace { image_path: String },
    FrameImageFace { tl_image_path: String, frame_width: f32, frame_height: f32 },
    SBox, VBox, HBox, FBox, Composite,
    ScrolledWindow, ScrollClip, VScrollbar, HScrollbar, ListBox, OptionMenu, ScaleBar,
    MBox, ToolTipFace, ProgressFace, SceneFace, MovieFace, IconBox, Unknown(String),
}

#[derive(Clone)]
pub struct Widget {
    pub kind: WidgetKind, pub name: String,
    pub has_script: bool, pub is_component: bool, pub can_focus: bool, pub focus: bool,
    pub visible: bool, pub visible_condition: i32, pub visible_compare: i32,
    pub geometry: (f32, f32, f32, f32), pub clip: bool, pub opacity: f32,
    pub children: Vec<Widget>,
    pub packable: bool, pub pack_side: i32, pub pack_expand_x: bool, pub pack_expand_y: bool,
    pub pack_fill_x: bool, pub pack_fill_y: bool, pub pack_shrink_x: bool, pub pack_shrink_y: bool,
    pub pack_alignment_x: f32, pub pack_alignment_y: f32,
    pub pack_pad_left: f32, pub pack_pad_right: f32, pub pack_pad_top: f32, pub pack_pad_bottom: f32,
    pub pack_allocate_w: bool, pub pack_allocate_h: bool,
    pub inner_pad_left: f32, pub inner_pad_right: f32, pub inner_pad_top: f32, pub inner_pad_bottom: f32,
    pub is_face: bool, pub computed: ComputedLayout,
    pub actors: Vec<ActorDef>,
}

#[derive(Clone, Default)]
pub struct ComputedLayout { pub x: f32, pub y: f32, pub w: f32, pub h: f32 }

#[derive(Clone)]
pub enum ActorDef {
    ScalarInterpolator { name: String, attr: String, loop_count: i32, reverse: bool, initial_playing: bool, sections: Vec<MScalarSection> },
    VectorInterpolator { name: String, attr: String, loop_count: i32, reverse: bool, initial_playing: bool, sections: Vec<MVectorSection> },
    ParallelActor { name: String, initial_pause: bool, children: Vec<ActorDef> },
    SequenceActor { name: String, children: Vec<ActorDef> },
    FaceColorInterpolator { name: String, attr: String, loop_count: i32, reverse: bool, initial_playing: bool, sections: Vec<MFaceColorSection> },
    MColorInterpolator { name: String, sections: Vec<MFaceColorSection> },
    Unknown(String),
}
#[derive(Clone)] pub struct MScalarSection { pub start_duration: f32, pub interp_period: f32, pub start_value: f32, pub end_value: f32 }
#[derive(Clone)] pub struct MVectorSection { pub interp_period: f32, pub start_value: (f32, f32), pub end_value: (f32, f32) }
#[derive(Clone)] pub struct MFaceColorSection { pub interp_period: f32, pub end_value: Vec<RGBA> }

#[derive(Clone)]
pub struct MProject {
    pub name: String, pub has_script: bool, pub focus: bool, pub root_windows: Vec<Widget>,
}

pub struct UiManager {
    pub active_project: Option<String>,
    pub project: Option<MProject>,
    pub finished: bool,
    pub focus_index: usize,
    pub focus_widgets: Vec<String>,
    pub frame_time: f32,
    pub active_page: String,
    actor_states: HashMap<String, ActorState>,
}
struct ActorState { time: f32, playing: bool, reversed: bool, done: bool }

impl UiManager {
    pub fn new() -> Self { UiManager {
        active_project: None, project: None, finished: false, focus_index: 0,
        focus_widgets: Vec::new(), frame_time: 0.016, actor_states: HashMap::new(),
        active_page: String::new(),
    }}

    pub fn start_project(&mut self, name: &str) {
        if self.active_project.as_deref() == Some(name) { return; }
        self.active_project = Some(name.to_string());
        self.project = load_parsed_project(name);
        self.finished = false;
        self.active_page = String::new();
    }

    pub fn update(&mut self, key_state: u32, dt: f32) {
        self.frame_time = dt;
        if self.project.is_none() { return; }
        
        // Build focusable widget list if empty
        if self.focus_widgets.is_empty() {
            self.scan_focusable_widgets();
        }
        
        // Handle navigation (UP/DOWN/LEFT/RIGHT)
        // Bit positions: 2=UP 3=RIGHT 4=DOWN 5=LEFT
        if (key_state >> 2) & 1 != 0 { self.navigate_focus("up"); }
        if (key_state >> 4) & 1 != 0 { self.navigate_focus("down"); }
        if (key_state >> 5) & 1 != 0 { self.navigate_focus("left"); }
        if (key_state >> 3) & 1 != 0 { self.navigate_focus("right"); }
        
        // Handle activation (Cross button = bit 10)
        if (key_state >> 10) & 1 != 0 { 
            self.activate_focused();
            self.finished = true; 
        }
        // Circle = bit 9 (cancel/back)
        if (key_state >> 9) & 1 != 0 { self.finished = true; }
    }
    
    fn scan_focusable_widgets(&mut self) {
        self.focus_widgets.clear();
        
        // Collect from all root windows
        if let Some(ref proj) = self.project {
            let windows = proj.root_windows.clone();
            for window in windows {
                self.collect_focusable_recursive(&window);
            }
        }
    }
    
    fn collect_focusable_recursive(&mut self, widget: &Widget) {
        if widget.can_focus || matches!(widget.kind, WidgetKind::OptionMenu | WidgetKind::ListBox) {
            self.focus_widgets.push(widget.name.clone());
        }
        for child in &widget.children {
            self.collect_focusable_recursive(child);
        }
    }
    
    fn navigate_focus(&mut self, direction: &str) {
        if self.focus_widgets.is_empty() { return; }
        
        match direction {
            "up" | "left" => {
                if self.focus_index > 0 {
                    self.focus_index -= 1;
                } else {
                    self.focus_index = self.focus_widgets.len() - 1;
                }
            }
            "down" | "right" => {
                if self.focus_index < self.focus_widgets.len() - 1 {
                    self.focus_index += 1;
                } else {
                    self.focus_index = 0;
                }
            }
            _ => {}
        }
        
        eprintln!("[UI] Focus moved {} to '{}' (index {} of {})", 
            direction, 
            self.focused_widget().unwrap_or_default(),
            self.focus_index,
            self.focus_widgets.len()
        );
    }
    
    fn activate_focused(&mut self) {
        if let Some(ref widget_name) = self.focused_widget() {
            eprintln!("[UI] Activated widget '{}'", widget_name);
            // TODO: Trigger widget's onActivate/onClick event
            // This would typically call a VM function or trigger a native callback
        }
    }

    pub fn is_finished(&self) -> bool { self.finished }
    pub fn focused_widget(&self) -> Option<String> { self.focus_widgets.get(self.focus_index).cloned() }
    pub fn reset_finished(&mut self) { self.finished = false; }

    pub fn drain_events(&mut self) -> Vec<(String, String)> { vec![] }

    pub fn go_to_page(&mut self, page: &str) {
        self.active_page = page.to_string();
        self.focus_index = 0;
        self.focus_widgets.clear();
    }
}

fn is_page_root(name: &str) -> bool {
    matches!(name, 
        "BranchRoot" | "GTTopRoot" | "TopRoot" | "OnlineRoot" | "SelectRoomRoot" |
        "DrivingModeRoot" | "CourseRoot" | "CarRoot" | "DealerRoot" |
        "BuyCarRoot" | "TradeRoot" | "TradeCarRoot" | "ShareRoot" |
        "LicenseRoot" | "LicenseCategoryRoot" | "LicenseMapRoot" | "LicenseFirstStage" | "LicenseSecondStage" |
        "StatusRoot" | "LogsRoot" | "ReplayRoot" | "GameDataEditRoot"
    )
}
fn is_shared_root(name: &str) -> bool {
    matches!(name, "DetailPopup" | "Tooltip" | "Pulldown" | "Slider" | "Buttons" | "ActorCollection")
}

pub fn load_parsed_project(name: &str) -> Option<MProject> {
    // Try to parse the .mproject, but if it fails, return a minimal project
    let paths = [
        format!("{}\\projects\\gt5m\\{}\\{}.mproject", assets_root(), name, name),
        format!("assets\\projects\\gt5m\\{}\\{}.mproject", name, name),
    ];
    for path in &paths {
        if let Ok(source) = std::fs::read_to_string(path) {
            let tokens = tokenize(&source);
            let mut parser = MProjectParser::new(tokens);
            let proj = parser.parse_project();
            eprintln!("[UI] Loaded project '{}' from {} ({} root windows, {} focus widgets)",
                proj.name, path, proj.root_windows.len(), 0);
            return Some(proj);
        }
    }
    eprintln!("[UI] Project '{}' not found, using minimal", name);
    // Minimal project with a simple RootWindow
    Some(MProject {
        name: name.to_string(), has_script: false, focus: false,
        root_windows: vec![Widget {
            kind: WidgetKind::RootWindow, name: name.to_string(),
            has_script: false, is_component: false, can_focus: false, focus: false,
            visible: true, visible_condition: 0, visible_compare: 0,
            geometry: (0.0, 0.0, 480.0, 272.0), clip: false, opacity: 1.0,
            children: vec![],
            packable: false, pack_side: 0, pack_expand_x: false, pack_expand_y: false,
            pack_fill_x: false, pack_fill_y: false, pack_shrink_x: false, pack_shrink_y: false,
            pack_alignment_x: 0.0, pack_alignment_y: 0.0,
            pack_pad_left: 0.0, pack_pad_right: 0.0, pack_pad_top: 0.0, pack_pad_bottom: 0.0,
            pack_allocate_w: false, pack_allocate_h: false,
            inner_pad_left: 0.0, inner_pad_right: 0.0, inner_pad_top: 0.0, inner_pad_bottom: 0.0,
            is_face: false, computed: ComputedLayout::default(), actors: vec![],
        }],
    })
}

// ─── Parser (depth-limited) ──────────────────────────────
fn parse_float(s: &str) -> f32 { s.trim().parse().unwrap_or(0.0) }
fn parse_int(s: &str) -> i32 { s.trim().parse().unwrap_or(0) }
fn parse_bool(s: &str) -> bool { parse_int(s) != 0 }

struct MProjectParser { tokens: Vec<String>, pos: usize, depth: u32 }
impl MProjectParser {
    fn new(t: Vec<String>) -> Self { MProjectParser { tokens: t, pos: 0, depth: 0 } }
    fn peek(&self) -> Option<&str> { self.tokens.get(self.pos).map(|s| s.as_str()) }
    fn consume(&mut self) -> Option<String> { let t = self.tokens.get(self.pos).cloned(); self.pos += 1; t }
    fn expect(&mut self, e: &str) -> bool { if self.peek() == Some(e) { self.pos += 1; true } else { false } }

    fn parse_project(&mut self) -> MProject {
        self.expect("Project"); self.expect("{");
        let mut name = String::new(); let mut has_script = false; let mut focus = false; let mut children = Vec::new();
        while let Some(tok) = self.peek() {
            if tok == "}" { self.pos += 1; break; }
            match tok {
                "name" => { self.pos += 1; name = self.consume().unwrap_or_default(); }
                "has_script" => { self.pos += 1; self.expect("digit"); self.expect("{"); has_script = self.consume().map(|s| parse_bool(&s)).unwrap_or(false); self.expect("}"); }
                "focus" => { self.pos += 1; self.expect("digit"); self.expect("{"); focus = self.consume().map(|s| parse_bool(&s)).unwrap_or(false); self.expect("}"); }
                "children" => { self.pos += 1; self.expect("["); self.consume(); self.expect("]"); self.expect("{"); children = self.parse_children(); self.expect("}"); }
                "project_component" => { self.pos += 1; let _ = self.parse_widget(); }
                _ => { self.pos += 1; }
            }
        }
        MProject { name, has_script, focus, root_windows: children }
    }

    fn parse_children(&mut self) -> Vec<Widget> {
        if self.depth > 40 { return vec![]; }
        self.depth += 1;
        let mut ws = Vec::new();
        while let Some(tok) = self.peek() { if tok == "}" { break; } ws.push(self.parse_widget()); }
        self.depth -= 1;
        ws
    }

    fn parse_widget(&mut self) -> Widget {
        let type_name = self.consume().unwrap_or_default();
        self.expect("{");
        let kind = Self::type_to_kind(&type_name);
        let mut w = Widget {
            kind, name: String::new(), has_script: false, is_component: false, can_focus: false, focus: false,
            visible: true, visible_condition: 0, visible_compare: 0,
            geometry: (0.0, 0.0, 0.0, 0.0), clip: false, opacity: 1.0, children: Vec::new(),
            packable: false, pack_side: 0, pack_expand_x: false, pack_expand_y: false,
            pack_fill_x: false, pack_fill_y: false, pack_shrink_x: false, pack_shrink_y: false,
            pack_alignment_x: 0.0, pack_alignment_y: 0.0,
            pack_pad_left: 0.0, pack_pad_right: 0.0, pack_pad_top: 0.0, pack_pad_bottom: 0.0,
            pack_allocate_w: true, pack_allocate_h: true,
            inner_pad_left: 0.0, inner_pad_right: 0.0, inner_pad_top: 0.0, inner_pad_bottom: 0.0,
            is_face: false, computed: ComputedLayout::default(), actors: Vec::new(),
        };
        while let Some(tok) = self.peek() {
            if tok == "}" { self.pos += 1; break; }
            self.parse_field(&mut w);
        }
        w
    }

    fn type_to_kind(n: &str) -> WidgetKind {
        match n {
            "RootWindow" => WidgetKind::RootWindow,
            "ColorFace" => WidgetKind::ColorFace { color: vec![], is_divide_color: false, y_divide: vec![], color_divide: vec![], opacity: 1.0 },
            "TextFace" => WidgetKind::TextFace { text: String::new(), font: "default".into(), color: RGBA{r:200,g:200,b:200,a:255}, shadow: RGBA{r:0,g:0,b:0,a:128}, shadow_value: 0, scale_x: 1.0, scale_y: 1.0, align: 3, multiline: false },
            "ImageFace" => WidgetKind::ImageFace { image_path: String::new() },
            "FrameImageFace" => WidgetKind::FrameImageFace { tl_image_path: String::new(), frame_width: 0.0, frame_height: 0.0 },
            "SBox" => WidgetKind::SBox, "VBox" => WidgetKind::VBox, "HBox" => WidgetKind::HBox,
            "FBox" => WidgetKind::FBox, "Composite" => WidgetKind::Composite,
            "ScrolledWindow" => WidgetKind::ScrolledWindow, "ScrollClip" => WidgetKind::ScrollClip,
            "VScrollbar" => WidgetKind::VScrollbar, "HScrollbar" => WidgetKind::HScrollbar,
            "ListBox" => WidgetKind::ListBox, "OptionMenu" => WidgetKind::OptionMenu,
            "ScaleBar" => WidgetKind::ScaleBar, "MBox" => WidgetKind::MBox,
            "ToolTipFace" => WidgetKind::ToolTipFace, "ProgressFace" => WidgetKind::ProgressFace,
            "SceneFace" => WidgetKind::SceneFace, "MovieFace" => WidgetKind::MovieFace,
            "IconBox" => WidgetKind::IconBox,
            _ => WidgetKind::Unknown(n.to_string()),
        }
    }

    fn parse_field(&mut self, w: &mut Widget) {
        let field = self.consume().unwrap_or_default();
        match field.as_str() {
            "name" => { w.name = self.consume().unwrap_or_default(); }
            "has_script" => { self.expect("digit"); self.expect("{"); w.has_script = self.consume().map(|s| parse_bool(&s)).unwrap_or(false); self.expect("}"); }
            "is_component" | "can_focus" | "focus" | "visible" | "clip" | "is_face" | "packable" | "pack_expand_x" | "pack_expand_y" | "pack_fill_x" | "pack_fill_y" | "pack_shrink_x" | "pack_shrink_y" | "pack_allocate_w" | "pack_allocate_h" | "multiline" | "adjust_scale" | "num_proportional" | "keep_scroll_point" | "adjust_popup_size" | "use_sync_timer" => {
                let b = parse_bool(&self.consume().unwrap_or_default());
                match field.as_str() {
                    "is_component" => w.is_component = b, "can_focus" => w.can_focus = b,
                    "focus" => w.focus = b, "visible" => w.visible = b,
                    "clip" => w.clip = b, "is_face" => w.is_face = b,
                    "packable" => w.packable = b, "pack_expand_x" => w.pack_expand_x = b,
                    "pack_expand_y" => w.pack_expand_y = b, "pack_fill_x" => w.pack_fill_x = b,
                    "pack_fill_y" => w.pack_fill_y = b, "pack_shrink_x" => w.pack_shrink_x = b,
                    "pack_shrink_y" => w.pack_shrink_y = b,
                    "pack_allocate_w" => w.pack_allocate_w = b,
                    "pack_allocate_h" => w.pack_allocate_h = b,
                    _ => {}
                }
                self.expect("}");
            }
            "visible_condition" | "visible_compare" | "pack_side" | "shadow_value" | "align" | "display_policy" | "loop_count" | "h_magnify" | "v_magnify" | "value" => {
                let v = parse_int(&self.consume().unwrap_or_default());
                match field.as_str() {
                    "visible_condition" => w.visible_condition = v, "visible_compare" => w.visible_compare = v,
                    "pack_side" => w.pack_side = v, "shadow_value" => { if let WidgetKind::TextFace { ref mut shadow_value, .. } = w.kind { *shadow_value = v; } }
                    "align" => { if let WidgetKind::TextFace { ref mut align, .. } = w.kind { *align = v; } }
                    _ => {}
                }
                self.expect("}");
            }
            "geometry" => {
                self.expect("rectangle"); self.expect("{");
                let x = parse_float(&self.consume().unwrap_or_default());
                let y = parse_float(&self.consume().unwrap_or_default());
                let gw = parse_float(&self.consume().unwrap_or_default());
                let gh = parse_float(&self.consume().unwrap_or_default());
                w.geometry = (x, y, gw, gh);
                self.expect("}");
            }
            "color" => { self.expect("["); self.consume(); self.expect("]"); self.expect("{"); let mut c = Vec::new(); while self.peek() != Some("}") { c.push(self.parse_rgba()); } self.expect("}"); if let WidgetKind::ColorFace { ref mut color, .. } = w.kind { *color = c; } }
            "text_color" => { let c = self.parse_rgba(); if let WidgetKind::TextFace { ref mut color, .. } = w.kind { *color = c; } }
            "shadow_color" => { let c = self.parse_rgba(); if let WidgetKind::TextFace { ref mut shadow, .. } = w.kind { *shadow = c; } }
            "text" | "font" | "key" | "localized_text_page" | "image_path" | "tl_image_path" | "r_image_path" | "blend_func_name" | "increase_mode_name" | "ease_type_name" | "attribute_name" | "focus_enter_action_name" | "focus_leave_action_name" => {
                let s = self.consume().unwrap_or_default();
                match field.as_str() {
                    "text" => { if let WidgetKind::TextFace { ref mut text, .. } = w.kind { *text = s; } }
                    "font" => { if let WidgetKind::TextFace { ref mut font, .. } = w.kind { *font = s; } }
                    "image_path" => {
                        match w.kind { WidgetKind::ImageFace { ref mut image_path } => *image_path = s, WidgetKind::FrameImageFace { ref mut tl_image_path, .. } => *tl_image_path = s, _ => {} }
                    }
                    "tl_image_path" => { if let WidgetKind::FrameImageFace { ref mut tl_image_path, .. } = w.kind { *tl_image_path = s; } }
                    _ => {}
                }
            }
            "children" => { self.expect("["); self.consume(); self.expect("]"); self.expect("{"); w.children = self.parse_children(); self.expect("}"); }
            "actor_list" => { self.expect("["); self.consume(); self.expect("]"); self.expect("{"); while self.peek() != Some("}") { self.parse_actor(); } self.expect("}"); }
            "section" | "color_divide" | "y_divide" | "is_divide_color" | "opacity" | "scale_x" | "scale_y" | "pack_alignment_x" | "pack_alignment_y" | "pack_pad_left" | "pack_pad_right" | "pack_pad_top" | "pack_pad_bottom" | "inner_pad_left" | "inner_pad_right" | "inner_pad_top" | "inner_pad_bottom" | "frame_width" | "frame_height" | "cursor_align_x" | "cursor_align_y" | "x_alignment" | "y_alignment" | "round" | "start_duration" | "interpolation_period" | "start_value" | "end_value" => { self.skip_block(); }
            "navigate_source" | "navigate_target" => { self.expect("region"); self.expect("{"); for _ in 0..4 { self.consume(); } self.expect("}"); }
            _ => {}
        }
    }

    fn parse_actor(&mut self) -> ActorDef {
        let type_name = self.consume().unwrap_or_default();
        self.expect("{");
        match type_name.as_str() {
            "ScalarInterpolator" | "MColorInterpolator" => {
                let mut name = String::new(); let mut attr = String::new();
                let mut loop_count = 0; let mut reverse = false;
                let mut initial_playing = false; let mut sections = vec![];
                while self.peek() != Some("}") {
                    let key = self.consume().unwrap_or_default();
                    match key.as_str() {
                        "name" => { self.expect("string"); name = self.consume().unwrap_or_default(); }
                        "attr" => { self.expect("string"); attr = self.consume().unwrap_or_default(); }
                        "loop_count" => { self.expect("digit"); loop_count = parse_int(&self.consume().unwrap_or_default()); }
                        "reverse" => { self.expect("digit"); reverse = parse_int(&self.consume().unwrap_or_default()) != 0; }
                        "initial_playing" => { self.expect("digit"); initial_playing = parse_int(&self.consume().unwrap_or_default()) != 0; }
                        "sections" => {
                            self.expect("["); self.consume(); self.expect("]"); self.expect("{");
                            while self.peek() != Some("}") {
                                let _sec_type = self.consume(); // MScalarSection / MFaceColorSection
                                self.expect("{");
                                let mut sd = 0.0f32; let mut ip = 0.0f32;
                                let mut sv = 0.0f32; let mut ev = 0.0f32;
                                while self.peek() != Some("}") {
                                    let k2 = self.consume().unwrap_or_default();
                                    match k2.as_str() {
                                        "start_duration" => { self.expect("float"); sd = parse_float(&self.consume().unwrap_or_default()); }
                                        "interpolation_period" => { self.expect("float"); ip = parse_float(&self.consume().unwrap_or_default()); }
                                        "start_value" => { self.expect("float"); sv = parse_float(&self.consume().unwrap_or_default()); }
                                        "end_value" => { self.expect("float"); ev = parse_float(&self.consume().unwrap_or_default()); }
                                        _ => { self.skip_block(); }
                                    }
                                }
                                self.expect("}");
                                sections.push(MScalarSection { start_duration: sd, interp_period: ip, start_value: sv, end_value: ev });
                            }
                            self.expect("}");
                        }
                        _ => { self.skip_block(); }
                    }
                }
                self.expect("}");
                ActorDef::ScalarInterpolator { name, attr, loop_count, reverse, initial_playing, sections }
            }
            _ => {
                self.skip_block(); self.expect("}");
                ActorDef::Unknown(type_name)
            }
        }
    }

    fn parse_rgba(&mut self) -> RGBA {
        self.expect("RGBA"); self.expect("{");
        let r = parse_int(&self.consume().unwrap_or_default()) as u8;
        let g = parse_int(&self.consume().unwrap_or_default()) as u8;
        let b = parse_int(&self.consume().unwrap_or_default()) as u8;
        let a = parse_int(&self.consume().unwrap_or_default()) as u8;
        self.expect("}");
        RGBA { r, g, b, a }
    }

    fn skip_block(&mut self) { let mut d = 1; while d > 0 { match self.peek() { Some("{") => { d+=1; self.pos+=1; } Some("}") => { d-=1; self.pos+=1; } Some(_) => { self.pos+=1; } None => break } } }
}

fn tokenize(source: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut i = 0;
    let chars: Vec<char> = source.chars().collect();
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() { i += 1; continue; }
        if c == '{' || c == '}' || c == '[' || c == ']' { tokens.push(c.to_string()); i += 1; continue; }
        if c == '"' || c == '\'' {
            let quote = c;
            i += 1; let mut s = String::new();
            while i < chars.len() && chars[i] != quote {
                if chars[i] == '\\' {
                    i += 1;
                    if i < chars.len() {
                        s.push(match chars[i] {
                            'n' => '\n',
                            't' => '\t',
                            'r' => '\r',
                            '\\' => '\\',
                            '"' => '"',
                            '\'' => '\'',
                            '0' => '\0',
                            other => other,
                        });
                    }
                } else { s.push(chars[i]); }
                i += 1;
            }
            i += 1; tokens.push(s); continue;
        }
        let mut word = String::new();
        while i < chars.len() && !chars[i].is_whitespace() && chars[i] != '{' && chars[i] != '}' && chars[i] != '[' && chars[i] != ']' { word.push(chars[i]); i += 1; }
        if !word.is_empty() { tokens.push(word); }
    }
    tokens
}
