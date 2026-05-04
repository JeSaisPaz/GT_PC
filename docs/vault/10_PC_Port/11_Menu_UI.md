---
tags: [pc-port, ui, menu, focus, navigation, widgets]
type: documentation
project: GT PSP PC Port
section: UI System
---

# Menu UI — Focus Navigation & Activation

> Complete widget focus system with keyboard/controller navigation for the GT PSP PC Port.

## Overview

The Menu UI system manages widget focus navigation, activation, and event handling for the PC port. It implements a full focus management system compatible with the original game's MWidget architecture.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     UiManager                                │
├─────────────────────────────────────────────────────────────┤
│  focus_widgets: Vec<String>     ← List of focusable widgets │
│  focus_index: usize             ← Currently focused index   │
│  active_project: Option<String> ← Current UI project        │
│  project: Option<MProject>      ← Parsed .mproject          │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│                   update(key_state, dt)                      │
├─────────────────────────────────────────────────────────────┤
│  1. scan_focusable_widgets()  ← Build focus list          │
│  2. navigate_focus(direction)   ← Handle UP/DOWN/LEFT/RIGHT  │
│  3. activate_focused()          ← Handle Cross button         │
└─────────────────────────────────────────────────────────────┘
```

## Focusable Widgets

Widgets are focusable if they have:
- `can_focus = true` flag, OR
- `WidgetKind::OptionMenu` — Dropdown menus, OR
- `WidgetKind::ListBox` — Scrollable lists

### Focus Discovery

```rust
fn scan_focusable_widgets(&mut self) {
    self.focus_widgets.clear();
    
    if let Some(ref proj) = self.project {
        let windows = proj.root_windows.clone();
        for window in windows {
            self.collect_focusable_recursive(&window);
        }
    }
}

fn collect_focusable_recursive(&mut self, widget: &Widget) {
    if widget.can_focus || matches!(widget.kind, 
        WidgetKind::OptionMenu | WidgetKind::ListBox) {
        self.focus_widgets.push(widget.name.clone());
    }
    for child in &widget.children {
        self.collect_focusable_recursive(child);
    }
}
```

## Navigation

### Input Mapping (PSP Buttons)

| Button | Bit Position | Action |
|--------|--------------|--------|
| UP | 2 | `navigate_focus("up")` |
| DOWN | 4 | `navigate_focus("down")` |
| LEFT | 5 | `navigate_focus("left")` |
| RIGHT | 3 | `navigate_focus("right")` |
| CROSS | 10 | `activate_focused()` + finish |
| CIRCLE | 9 | finish (cancel/back) |

### Circular Navigation

Navigation wraps around at list boundaries:

```rust
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
```

## Activation

### Cross Button Handling

When CROSS is pressed on a focused widget:

```rust
fn activate_focused(&mut self) {
    if let Some(ref widget_name) = self.focused_widget() {
        eprintln!("[UI] Activated widget '{}'", widget_name);
        // TODO: Trigger widget's onActivate/onClick event
        // This would typically call a VM function or trigger a native callback
    }
}
```

### Current Implementation

The activation currently:
1. Logs the activated widget name
2. Sets `finished = true` to exit the menu loop
3. Future: Will trigger VM callbacks for widget events

## Update Loop

The `UiManager::update()` method processes input each frame:

```rust
pub fn update(&mut self, key_state: u32, dt: f32) {
    self.frame_time = dt;
    if self.project.is_none() { return; }
    
    // Build focusable widget list if empty
    if self.focus_widgets.is_empty() {
        self.scan_focusable_widgets();
    }
    
    // Handle navigation (UP/DOWN/LEFT/RIGHT)
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
```

## Widget Types

### Focusable Widgets

| WidgetKind | Can Focus | Description |
|------------|-----------|-------------|
| `OptionMenu` | ✅ | Dropdown selection menus |
| `ListBox` | ✅ | Scrollable item lists |
| `RootWindow` | ❌ | Container (not interactive) |
| `ColorFace` | ❌ | Background color (decorative) |
| `TextFace` | ❌ | Static text (display only) |
| `ImageFace` | ❌ | Static image (display only) |
| `MBox` | ❌ | Message box (passive) |

## Future Enhancements

### Planned Features

1. **Visual Focus Indicator**: Highlight border/glow around focused widget
2. **Scroll Into View**: Auto-scroll when focusing off-screen widgets
3. **Animation**: Smooth focus transitions
4. **Sound Feedback**: Audio cue on focus change and activation
5. **VM Integration**: Trigger `onActivate`, `onFocusEnter`, `onFocusLeave` callbacks

### VM Callback Integration

```rust
// Future: Trigger VM function for widget events
fn trigger_widget_callback(&mut self, widget_name: &str, event: &str) {
    let callback_path = format!("main,menu,{},on{}", widget_name, event);
    if let Some(func) = vm_engine.global_functions.get(&callback_path) {
        vm_engine.call_function_value(func.clone(), vec![]);
    }
}
```

## Source Files

| File | Purpose |
|------|---------|
| `pc_port/src/engine/ui.rs` | UiManager, focus navigation, widget collection |
| `pc_port/src/engine/menu.rs` | Button input mapping, menu state |

## See Also

- [[10_PC_Port/00_Index|PC Port Index]]
- [[10_PC_Port/06_Native_API|Native API]]
- [[20_ADHOC_VM/00_Index|Adhoc VM]]
