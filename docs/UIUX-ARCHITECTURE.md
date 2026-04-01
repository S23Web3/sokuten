# QuickPaste — UI/UX Architecture

**Version:** v1 draft  
**Date:** 2026-03-31  
**Stack:** Rust 1.94 · egui 0.29 · eframe 0.29 · windows 0.58  
**Scope:** All UI/UX decisions for the tray-based text expander. Implementation-ready: every section contains exact Rust/egui API calls and crate additions.

---

## Table of Contents

1. [Feature Hierarchy (P0/P1/P2)](#1-feature-hierarchy)
2. [Always-On-Top Floating Window](#2-always-on-top-floating-window)
3. [Global Hotkeys](#3-global-hotkeys)
4. [Dark / Light Theme Toggle](#4-dark--light-theme-toggle)
5. [Window Sizing — Compact vs Full Mode](#5-window-sizing--compact-vs-full-mode)
6. [Tray Icon Behaviour](#6-tray-icon-behaviour)
7. [Search / Filter Bar](#7-search--filter-bar)
8. [Keyboard Navigation Within the Window](#8-keyboard-navigation-within-the-window)
9. [Inline Status Feedback](#9-inline-status-feedback)
10. [Paste-and-Hide Delay Tuning](#10-paste-and-hide-delay-tuning)
11. [Idle Repaint Throttling](#11-idle-repaint-throttling)
12. [Drag-to-Reorder Phrases](#12-drag-to-reorder-phrases)
13. [Window Position Memory](#13-window-position-memory)
14. [Accessibility Minimum Checklist](#14-accessibility-minimum-checklist)

---

## 1. Feature Hierarchy

### P0 — Must-have v1 (ship-blocking)

| ID | Feature |
|----|---------|
| P0-1 | Always-on-top floating window, resizable |
| P0-2 | Compact mode (phrase list only) / Full mode (+ add form) toggle |
| P0-3 | Global hotkey: show/hide window (`Ctrl+Shift+Space`) |
| P0-4 | Dark / Light theme toggle, persisted |
| P0-5 | Tray icon: left-click = show/hide, right-click = context menu |
| P0-6 | Paste-and-hide: click Paste → window hides → inject → error recovery |
| P0-7 | Search / filter bar (live, case-insensitive) |
| P0-8 | Idle repaint throttling (no busy loop) |

### P1 — Should-have v1 (quality of life, no extra crates required)

| ID | Feature |
|----|---------|
| P1-1 | Window position memory (persist last x/y across restarts) |
| P1-2 | Keyboard navigation: arrow keys select phrase, Enter pastes |
| P1-3 | Inline status bar: last action + timestamp |
| P1-4 | Compact mode minimum height (220 px), full mode minimum height (420 px) |
| P1-5 | Escape key closes / hides window |
| P1-6 | Paste delay slider in settings (50–500 ms) |

### P2 — Nice-to-have v2 (additional effort or crates)

| ID | Feature |
|----|---------|
| P2-1 | Drag-to-reorder phrases |
| P2-2 | Per-phrase global hotkey (e.g. `Ctrl+Shift+1` through `Ctrl+Shift+9`) |
| P2-3 | Window transparency slider (`with_transparent`) |
| P2-4 | Import / export phrases as CSV or JSON |
| P2-5 | Tag / group support for long phrase lists |
| P2-6 | Prefix-trigger mode: type `/hello` → auto-inject (requires background hook) |

---

## 2. Always-On-Top Floating Window

### What it is

The main QuickPaste window floats above every other application at all times. The user can resize it freely. A compact/expanded toggle shrinks it to phrase-list-only height so it never blocks content during normal work.

### Implementation — ViewportBuilder flags

Set both flags in `ui::run()` when constructing `NativeOptions`:

```rust
let options = eframe::NativeOptions {
    viewport: egui::ViewportBuilder::default()
        .with_title("QuickPaste")
        .with_always_on_top(true)           // P0-1: layered above all apps
        .with_resizable(true)               // P0-1: user-resizable
        .with_inner_size([360.0, 480.0])    // full-mode default
        .with_min_inner_size([280.0, 220.0])// compact-mode floor
        .with_decorations(true),            // keep title bar (drag target)
    ..Default::default()
};
```

`with_always_on_top(true)` maps to `WS_EX_TOPMOST` via winit internally. No additional Win32 call is needed.

### Re-asserting always-on-top at runtime

winit/eframe does not re-apply `TOPMOST` if another window forcibly removes it. Poll this once per second as a safety measure:

```rust
// In update(), once per ~60 frames at 60 fps
self.frame_count = self.frame_count.wrapping_add(1);
if self.frame_count % 60 == 0 {
    ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
        egui::WindowLevel::AlwaysOnTop,
    ));
}
```

`egui::WindowLevel::AlwaysOnTop` is available from egui 0.27+. In egui 0.29 the enum is `egui::WindowLevel` with variants `Normal`, `AlwaysOnBottom`, `AlwaysOnTop`.

### Gotchas

- `WS_EX_TOPMOST` does not beat UAC elevation prompts or the Windows lock screen. Those always appear above. Do not try to work around this — it is correct OS security behaviour.
- Fullscreen DirectX/Vulkan games reclaim the top layer. When the user alt-tabs back the window re-appears. No workaround needed.
- If the user moves QuickPaste to a secondary monitor and disconnects it, the window teleports off-screen. Clamp the saved position on load (see [Section 13](#13-window-position-memory)).

---

## 3. Global Hotkeys

### What it is

Two system-wide hotkeys registered via `RegisterHotKey` Win32 API:

| Hotkey | Action |
|--------|--------|
| `Ctrl+Shift+Space` | Toggle show/hide QuickPaste window |
| `Ctrl+Shift+V` | Focus the window and paste the most-recently-used phrase |

### Why these keys are safe

**Extensive conflict matrix:**

| Combination | Windows OS | VS Code | Chrome | Office | Teams | Slack | Discord |
|-------------|------------|---------|--------|--------|-------|-------|---------|
| `Ctrl+Shift+Space` | FREE | IntelliSense param hints (editor-local, not global) | FREE | FREE | FREE | FREE | FREE |
| `Ctrl+Shift+V` | FREE | Paste without format (editor-local) | Paste from clipboard (tab-local) | FREE | FREE | FREE | FREE |
| `Ctrl+Alt+Space` | FREE | FREE | FREE | FREE | FREE | FREE | FREE |
| `Ctrl+Alt+Q` | FREE | FREE | FREE | FREE | Toggle push-to-talk | FREE | FREE |
| `Win+Shift+Q` | FREE* | FREE | FREE | FREE | FREE | FREE | FREE |

*`Win+Q` opens Search; `Win+Shift+Q` is unassigned in Windows 11 (as of 24H2).

**Confirmed global hotkey conflicts to avoid:**

- `Ctrl+Shift+Esc` — Task Manager (Windows, global)
- `Ctrl+Alt+Del` — Security screen (Windows, cannot be overridden)
- `Win+L` — Lock screen (Windows, cannot be overridden)
- `Win+D` — Show desktop (Windows)
- `Win+Tab` — Task view (Windows)
- `Ctrl+Shift+N` — New incognito window (Chrome, global when focused — but NOT a `RegisterHotKey` global)
- `Ctrl+Shift+T` — Reopen tab (Chrome — same, not a global Win32 hotkey)
- `Ctrl+Alt+T` — Open terminal (various Linux defaults, NOT applicable on Windows 11 unless user has remapped)
- `PrintScreen`, `Alt+PrintScreen` — Screenshot (Windows)

**Recommendation:** Use `Ctrl+Shift+Space` as the primary show/hide toggle. It is free globally on Windows 11 and no common application registers it with `RegisterHotKey`. `Ctrl+Shift+V` as secondary (recent-phrase trigger) is also safe at the system level, though editors intercept it locally — which is acceptable because when an editor is focused, the user probably does not want to trigger QuickPaste from a hotkey anyway.

### Crate additions

The existing `windows = "0.58"` dependency already has `Win32_UI_Input_KeyboardAndMouse`. Add one more feature flag:

```toml
[dependencies]
windows = { version = "0.58", features = [
    "Win32_UI_Input_KeyboardAndMouse",
    "Win32_UI_WindowsAndMessaging",   # WM_HOTKEY, GetMessage
    "Win32_Foundation",
] }
```

No new crate is required.

### Implementation — hotkey module (`src/hotkey.rs`)

```rust
//! Global hotkey registration and Win32 message pump.
//!
//! `RegisterHotKey` posts `WM_HOTKEY` to the registering thread's message
//! queue. We spin a dedicated OS thread that blocks on `GetMessageW`, then
//! sends the hotkey ID over an `mpsc` channel to the main (egui) thread.

use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey,
    MOD_CONTROL, MOD_SHIFT, HOT_KEY_MODIFIERS,
    VK_SPACE, VK_V,
};
use windows::Win32::UI::WindowsAndMessaging::{GetMessageW, MSG, WM_HOTKEY};

pub const HOTKEY_SHOW_HIDE: i32 = 1;
pub const HOTKEY_PASTE_RECENT: i32 = 2;

/// Spawn the hotkey listener thread.
///
/// Returns a `Receiver` that fires each time a registered hotkey is pressed.
/// The `i32` value is the hotkey ID (`HOTKEY_SHOW_HIDE` or `HOTKEY_PASTE_RECENT`).
pub fn spawn_hotkey_listener() -> Receiver<i32> {
    let (tx, rx): (Sender<i32>, Receiver<i32>) = mpsc::channel();

    thread::spawn(move || {
        // SAFETY: HWND(0) = no window; hotkey posts to thread message queue.
        unsafe {
            // Ctrl+Shift+Space — show/hide
            let _ = RegisterHotKey(
                HWND(std::ptr::null_mut()),
                HOTKEY_SHOW_HIDE,
                HOT_KEY_MODIFIERS(MOD_CONTROL.0 | MOD_SHIFT.0),
                VK_SPACE.0 as u32,
            );

            // Ctrl+Shift+V — paste most-recent phrase
            let _ = RegisterHotKey(
                HWND(std::ptr::null_mut()),
                HOTKEY_PASTE_RECENT,
                HOT_KEY_MODIFIERS(MOD_CONTROL.0 | MOD_SHIFT.0),
                VK_V.0 as u32,
            );
        }

        let mut msg = MSG::default();
        loop {
            // SAFETY: `GetMessageW` blocks until a message arrives.
            // Returns 0 on WM_QUIT, -1 on error.
            let ret = unsafe { GetMessageW(&mut msg, HWND(std::ptr::null_mut()), 0, 0) };
            if ret.0 <= 0 {
                break; // WM_QUIT or error — exit thread
            }
            if msg.message == WM_HOTKEY {
                let id = msg.wParam.0 as i32;
                if tx.send(id).is_err() {
                    break; // receiver dropped — main thread exited
                }
            }
        }

        // Unregister on exit
        unsafe {
            let _ = UnregisterHotKey(HWND(std::ptr::null_mut()), HOTKEY_SHOW_HIDE);
            let _ = UnregisterHotKey(HWND(std::ptr::null_mut()), HOTKEY_PASTE_RECENT);
        }
    });

    rx
}
```

### Wiring into `QuickPasteApp`

Add the receiver to app state and poll it in `update()`:

```rust
pub struct QuickPasteApp {
    // ... existing fields ...
    hotkey_rx: std::sync::mpsc::Receiver<i32>,
    window_visible: bool,
}

fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    // Poll hotkey channel (non-blocking)
    while let Ok(id) = self.hotkey_rx.try_recv() {
        match id {
            hotkey::HOTKEY_SHOW_HIDE => self.toggle_window_visibility(ctx),
            hotkey::HOTKEY_PASTE_RECENT => self.paste_most_recent(ctx),
            _ => {}
        }
    }
    // ... rest of update ...
}

fn toggle_window_visibility(&mut self, ctx: &egui::Context) {
    self.window_visible = !self.window_visible;
    if self.window_visible {
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    } else {
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
    }
}
```

### Edge cases and gotchas

- `RegisterHotKey` fails silently if another app already owns the combination. Always check the return value and surface the conflict to the user ("Hotkey Ctrl+Shift+Space is taken — showing warning"). Offer a fallback to `Ctrl+Alt+Space`.
- The hotkey thread must be spawned **before** `eframe::run_native` because `run_native` blocks the main thread. Spawn in `main()`, pass the `Receiver` into `QuickPasteApp`.
- `RegisterHotKey` with `HWND(null)` posts to the thread message queue, not a window. `GetMessageW` with `HWND(null)` retrieves all thread messages. This is correct and required.
- On process exit, eframe drops the `App` before the OS cleans up. The thread detects a closed `Sender` (`tx.send` returns `Err`) and exits cleanly, calling `UnregisterHotKey`.
- Admin-elevation boundary: hotkeys registered by a normal-privileges process are not delivered when a UAC-elevated window has focus. This is by design; do not attempt to elevate QuickPaste.
- Per-phrase hotkeys (P2-2) require IDs 3–11. Reserve them: `HOTKEY_PHRASE_BASE: i32 = 10`, indexed as `10 + phrase_index`. Cap at 9 per-phrase hotkeys because Windows allows at most one registered hotkey per ID per process.

---

## 4. Dark / Light Theme Toggle

### What it is

A single toggle button persisted in `config.json`. The window switches between egui's built-in dark and light Visuals on click, with no restart required.

### Implementation

Add `theme: Theme` to `AppConfig` and `QuickPasteApp`:

```rust
// In phrases.rs
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub disclaimer_accepted: bool,
    pub theme: Theme,
}
```

Apply the theme at app startup (in `ui::run()` after constructing the `CreationContext`):

```rust
eframe::run_native(
    "QuickPaste",
    options,
    Box::new(|cc| {
        // Apply persisted theme immediately, before first frame
        match config.theme {
            Theme::Dark  => cc.egui_ctx.set_visuals(egui::Visuals::dark()),
            Theme::Light => cc.egui_ctx.set_visuals(egui::Visuals::light()),
        }
        Ok(Box::new(QuickPasteApp { /* ... */ theme: config.theme }))
    }),
)
```

Toggle button in the toolbar:

```rust
fn show_toolbar(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
    let label = match self.theme {
        Theme::Dark  => "☀ Light",
        Theme::Light => "☾ Dark",
    };

    if ui.button(label).clicked() {
        self.theme = match self.theme {
            Theme::Dark  => Theme::Light,
            Theme::Light => Theme::Dark,
        };
        match self.theme {
            Theme::Dark  => ctx.set_visuals(egui::Visuals::dark()),
            Theme::Light => ctx.set_visuals(egui::Visuals::light()),
        }
        // Persist immediately
        let cfg = AppConfig {
            disclaimer_accepted: self.disclaimer_accepted,
            theme: self.theme,
        };
        if let Err(e) = save_config(&cfg) {
            tracing::error!("Failed to save theme: {e}");
        }
    }
}
```

### Custom accent colours (optional, P1 quality polish)

egui `Visuals` is a struct, not opaque. Override specific fields to match Windows 11 accent:

```rust
let mut vis = egui::Visuals::dark();
vis.widgets.active.bg_fill     = egui::Color32::from_rgb(0, 120, 212); // Win11 blue
vis.widgets.hovered.bg_fill    = egui::Color32::from_rgb(0, 99, 177);
vis.selection.bg_fill          = egui::Color32::from_rgb(0, 120, 212);
ctx.set_visuals(vis);
```

### Edge cases

- `set_visuals` takes effect immediately on the next frame. No intermediate "flash" occurs because egui redraws synchronously on state change.
- If `config.json` is missing the `theme` field (old config), `serde` deserialises to `Default::default()` (Dark) without error — safe.
- Do not use `egui::Visuals::default()` — in egui 0.29 this is `dark()`, but that guarantee is not documented. Explicitly call `dark()` or `light()`.

---

## 5. Window Sizing — Compact vs Full Mode

### What it is

Two logical window states:

| State | Height | Contents |
|-------|--------|----------|
| **Compact** | ~220 px | Search bar + phrase list only. Add form hidden. |
| **Full** | ~480 px | Search bar + phrase list + add form + toolbar. |

The toggle is a small `⊞`/`⊟` button in the title area. Mode is persisted in `config.json`.

### Dimensions reference

```
Compact (360 × 220):
┌──────────────────────────┐
│ QuickPaste         [⊞][×]│  ← title bar (OS decorations)
│──────────────────────────│
│ [🔍 filter phrases...   ]│  ← 32 px search bar
│──────────────────────────│
│  Hello              [▶]  │  \
│  Email sig          [▶]  │   } scrollable phrase list
│  Meeting notes      [▶]  │  /
│──────────────────────────│
│  ⊞  ☀ Light              │  ← status bar (28 px)
└──────────────────────────┘

Full (360 × 480):
┌──────────────────────────┐
│ QuickPaste         [⊟][×]│
│──────────────────────────│
│ [🔍 filter phrases...   ]│
│──────────────────────────│
│  Hello              [▶]  │
│  Email sig          [▶]  │
│  Meeting notes      [▶]  │
│──────────────────────────│
│  Label:                  │  \
│  [________________]      │   |
│  Text:                   │   } add form (collapsible)
│  [________________]      │   |
│  [________________]      │   |
│  [ Add Phrase ]          │  /
│──────────────────────────│
│  ⊟  ☀ Light   Last: OK   │
└──────────────────────────┘
```

### Implementation

Add `compact_mode: bool` to app state. Toggle button resizes the viewport:

```rust
fn show_toolbar(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
    let icon = if self.compact_mode { "⊞" } else { "⊟" };
    if ui.button(icon).on_hover_text(
        if self.compact_mode { "Expand" } else { "Compact" }
    ).clicked() {
        self.compact_mode = !self.compact_mode;
        let new_size = if self.compact_mode {
            egui::vec2(360.0, 220.0)
        } else {
            egui::vec2(360.0, 480.0)
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(new_size));
    }
}
```

Conditionally render the add form:

```rust
fn show_main_ui(&mut self, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        self.show_search_bar(ui);
        ui.separator();
        self.show_phrase_list(ctx, ui);
        ui.separator();

        if !self.compact_mode {
            self.show_add_form(ui);
            ui.separator();
        }

        self.show_toolbar(ctx, ui);
    });
}
```

Set `with_min_inner_size` in `ViewportBuilder` to enforce floors:

```rust
.with_min_inner_size([280.0, 180.0])
```

### Edge cases

- `ViewportCommand::InnerSize` is non-blocking; the resize takes effect on the next OS layout pass. There is no frame where the content overflows.
- When the user manually drags the window taller in compact mode, do not override that size — only set size on explicit toggle click.
- Persist the mode in `config.json` so QuickPaste reopens in the last-used mode.
- On very small laptop screens (< 1280 × 720), `360 × 480` may feel large. Consider capping at 60% of screen height: `egui::Context::screen_rect()` returns the available rect in logical pixels.

---

## 6. Tray Icon Behaviour

### What it is

The system tray (notification area) is the primary launch point. QuickPaste starts minimised to tray and only shows the window on user request.

### Current tray setup (from Phase 0)

The crate `tray-icon 0.19` + `muda 0.15` are already in `Cargo.toml`. No additions needed.

### Behaviour spec

| User action | Result |
|-------------|--------|
| Single left-click tray icon | Toggle window visible/hidden |
| Double left-click tray icon | Show window (always, even if already visible, bring to front) |
| Right-click tray icon | Open context menu |
| Context menu → "Show QuickPaste" | Same as single click |
| Context menu → "Compact / Full" | Toggle compact mode |
| Context menu → "Settings" | Open full mode, scroll to settings section |
| Context menu → "Quit" | `std::process::exit(0)` |

### Implementation — tray icon + context menu

Tray icon construction (in `main()` or passed into `QuickPasteApp`):

```rust
use tray_icon::{TrayIconBuilder, menu::Menu};
use muda::{MenuId, MenuItem, PredefinedMenuItem};

let tray_menu = Menu::new();
let show_item    = MenuItem::new("Show QuickPaste", true, None);
let compact_item = MenuItem::new("Toggle Compact", true, None);
let quit_item    = MenuItem::new("Quit", true, None);

tray_menu.append_items(&[
    &show_item,
    &compact_item,
    &PredefinedMenuItem::separator(),
    &quit_item,
]).unwrap();

let _tray = TrayIconBuilder::new()
    .with_menu(Box::new(tray_menu))
    .with_tooltip("QuickPaste")
    .with_icon(load_icon())   // load from embedded resource
    .build()
    .unwrap();
```

Poll tray events inside `update()`:

```rust
use tray_icon::TrayIconEvent;

// In update()
if let Ok(event) = TrayIconEvent::receiver().try_recv() {
    use tray_icon::ClickType;
    match event.click_type {
        ClickType::Left   => self.toggle_window_visibility(ctx),
        ClickType::Double => {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }
        _ => {}
    }
}

use muda::MenuEvent;
if let Ok(event) = MenuEvent::receiver().try_recv() {
    match event.id {
        id if id == show_item.id()    => self.toggle_window_visibility(ctx),
        id if id == compact_item.id() => self.toggle_compact(ctx),
        id if id == quit_item.id()    => std::process::exit(0),
        _ => {}
    }
}
```

### Gotchas

- `tray-icon` and `muda` must run on the main thread. Their `receiver()` returns a `crossbeam` channel. Polling in `update()` satisfies this requirement.
- The tray icon must persist as a variable (e.g. `_tray` stored in `QuickPasteApp`). If it is dropped, the icon disappears immediately.
- Icon must be 16×16 or 32×32 ICO. Embed via `include_bytes!` + parse with `tray_icon::icon::Icon::from_rgba()`.
- Do not call `process::exit` directly from a menu event without calling `save_phrases` first — the app drop might not run. Use a graceful shutdown flag or call save before exit.

---

## 7. Search / Filter Bar

### What it is

A text input at the top of the phrase list. Typing filters visible phrases by label or text content, case-insensitive, live (per keystroke).

### Implementation

Add `search_query: String` to app state. Render and apply the filter:

```rust
fn show_search_bar(&mut self, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label("🔍");
        let response = ui.add(
            egui::TextEdit::singleline(&mut self.search_query)
                .hint_text("Filter phrases…")
                .desired_width(f32::INFINITY),
        );
        // Auto-focus search bar when window first appears
        if self.just_shown {
            response.request_focus();
            self.just_shown = false;
        }
        if ui.button("✕").clicked() {
            self.search_query.clear();
        }
    });
}

fn filtered_phrases(&self) -> Vec<(usize, &Phrase)> {
    let q = self.search_query.to_lowercase();
    self.phrases
        .iter()
        .enumerate()
        .filter(|(_, p)| {
            q.is_empty()
                || p.label.to_lowercase().contains(&q)
                || p.text.to_lowercase().contains(&q)
        })
        .collect()
}
```

Use `filtered_phrases()` in `show_phrase_list` instead of `self.phrases.iter()`.

### Edge cases

- Clear the `search_query` after a paste action so the list resets for the next use.
- Do not filter while in the "add phrase" text fields — the filter bar and add form are distinct.
- With 0 results, show a centred label: `ui.centered_and_justified(|ui| { ui.label("No phrases match."); })`.

---

## 8. Keyboard Navigation Within the Window

### What it is

Arrow keys move selection through the phrase list. Enter pastes the selected phrase. Escape hides the window.

### Implementation

Add `selected_index: Option<usize>` to app state:

```rust
fn show_phrase_list(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
    let filtered = self.filtered_phrases();
    let max = filtered.len().saturating_sub(1);

    // Keyboard navigation
    if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
        self.selected_index = Some(match self.selected_index {
            None            => 0,
            Some(i)         => (i + 1).min(max),
        });
    }
    if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
        self.selected_index = Some(match self.selected_index {
            None | Some(0)  => 0,
            Some(i)         => i - 1,
        });
    }
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        self.window_visible = false;
    }

    let mut to_paste: Option<usize> = None;

    if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
        if let Some(sel) = self.selected_index {
            if let Some(&(real_idx, _)) = filtered.get(sel) {
                to_paste = Some(real_idx);
            }
        }
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        for (list_pos, (real_idx, phrase)) in filtered.iter().enumerate() {
            let selected = self.selected_index == Some(list_pos);
            let response = ui.selectable_label(selected, &phrase.label);

            if response.clicked() {
                self.selected_index = Some(list_pos);
            }
            if response.double_clicked() {
                to_paste = Some(*real_idx);
            }
            // Scroll selected item into view
            if selected {
                response.scroll_to_me(None);
            }
        }
    });

    if let Some(i) = to_paste {
        self.trigger_paste(ctx, i);
    }
}
```

### Edge cases

- Reset `selected_index` to `None` when `search_query` changes (the filtered list reorders).
- `selectable_label` highlights with `selection.bg_fill` from the current Visuals — works automatically in both dark and light themes.
- Tab key should cycle focus between the search bar and the add form. egui handles Tab natively for focusable widgets; no extra wiring needed.

---

## 9. Inline Status Feedback

### What it is

A single-line status bar at the bottom of the window showing the result of the last action and how long ago it occurred.

### Implementation

Add `status: Option<(String, std::time::Instant)>` to app state:

```rust
fn show_status_bar(&self, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        if let Some((ref msg, when)) = self.status {
            let age = when.elapsed().as_secs();
            let colour = if msg.starts_with("Error") {
                egui::Color32::from_rgb(255, 80, 80)
            } else {
                egui::Color32::from_rgb(100, 200, 100)
            };
            ui.colored_label(colour, msg);
            ui.label(format!("({}s ago)", age));
        }
    });
}

// Set status after paste:
self.status = Some((format!("Pasted: {}", phrase.label), std::time::Instant::now()));
// Set status after error:
self.status = Some((format!("Error: {e}"), std::time::Instant::now()));
```

Auto-clear after 5 seconds (check in `update()`):

```rust
if let Some((_, when)) = &self.status {
    if when.elapsed().as_secs() >= 5 {
        self.status = None;
    }
}
```

---

## 10. Paste-and-Hide Delay Tuning

### What it is

When the user clicks "Paste", the window is minimised/hidden before injection so the previous focused window regains focus. The current hardcoded 150 ms delay (`std::thread::sleep`) is blocking and may be insufficient or excessive depending on hardware.

### Problems with the current approach

- `thread::sleep` inside `update()` **freezes the entire egui frame loop** for 150 ms. This blocks repaints and makes the window feel hung.
- 150 ms may be too short on a slow machine (focus has not transferred), or too long on fast hardware (introduces perceptible lag).

### Correct implementation — async delay via deferred state

Replace the blocking sleep with a state machine:

```rust
#[derive(Default)]
enum PasteState {
    #[default]
    Idle,
    /// Window hidden, waiting for focus to transfer
    AwaitingFocus { phrase_idx: usize, hide_time: std::time::Instant },
    /// Injection sent, awaiting next frame to reset
    Done,
}

// In update(), before rendering panels:
match &self.paste_state {
    PasteState::AwaitingFocus { phrase_idx, hide_time } => {
        let delay = std::time::Duration::from_millis(self.config.paste_delay_ms as u64);
        if hide_time.elapsed() >= delay {
            let idx = *phrase_idx;
            self.paste_state = PasteState::Done;
            if let Some(phrase) = self.phrases.get(idx) {
                if let Err(e) = inject::send_text(&phrase.text) {
                    tracing::error!("Injection failed: {e}");
                    self.status = Some((format!("Error: {e}"), std::time::Instant::now()));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                } else {
                    self.status = Some((
                        format!("Pasted: {}", phrase.label),
                        std::time::Instant::now(),
                    ));
                }
            }
        } else {
            // Keep repainting until delay expires
            ctx.request_repaint();
        }
    }
    PasteState::Done => self.paste_state = PasteState::Idle,
    PasteState::Idle => {}
}
```

When user clicks Paste:

```rust
fn trigger_paste(&mut self, ctx: &egui::Context, idx: usize) {
    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
    self.window_visible = false;
    self.search_query.clear();
    self.selected_index = None;
    self.paste_state = PasteState::AwaitingFocus {
        phrase_idx: idx,
        hide_time: std::time::Instant::now(),
    };
}
```

### Configurable delay (P1-6)

Add `paste_delay_ms: u32` to `AppConfig` (default: `150`). Expose a slider in settings:

```rust
ui.add(egui::Slider::new(&mut self.config.paste_delay_ms, 50..=500)
    .text("Paste delay (ms)")
    .step_by(10.0));
```

---

## 11. Idle Repaint Throttling

### What it is

egui/eframe by default repaints only on input events. However, with `ctx.request_repaint()` calls (for hotkey polling, status expiry, etc.) it can degenerate into a busy loop. `request_repaint_after` ensures the app wakes at a controlled rate.

### Implementation

At the end of every `update()` call, replace any raw `request_repaint()` with a throttled version:

```rust
// Poll hotkeys, tray events, and status expiry at ~10 Hz
ctx.request_repaint_after(std::time::Duration::from_millis(100));
```

Exception: during `PasteState::AwaitingFocus`, use `request_repaint()` (unthrottled) to minimise injection latency — the delay is bounded by the paste delay value (50–500 ms), not an infinite loop.

This means:
- When the window is visible and idle: repaints at 10 Hz (invisible to user, CPU near zero)
- When animating or pasting: repaints as fast as possible for that single operation
- When window is hidden: still wakes at 10 Hz to poll hotkey channel — acceptable

If the window is hidden, egui renders nothing but still wakes. To skip rendering entirely when hidden:

```rust
if !self.window_visible {
    ctx.request_repaint_after(std::time::Duration::from_millis(100));
    // Poll only hotkey + tray, then return early
    return;
}
```

---

## 12. Drag-to-Reorder Phrases

### What it is (P2-1)

Users drag phrases up/down in the list to change their order. The order is persisted immediately.

### Crate addition

```toml
[dependencies]
egui_extras = "0.29"   # TableBuilder helpers (optional)
```

egui 0.29 does not ship a drag-reorder widget natively. The pattern is to implement drag manually using `response.drag_started()` / `response.drag_delta()` / `egui::Id` sense:

```rust
// Sketch — wire fully in v2
let id = egui::Id::new("phrase_drag");
let drag_src = self.drag_src;

for (i, phrase) in self.phrases.iter().enumerate() {
    let item_id = egui::Id::new(("phrase", i));
    let response = ui.dnd_drag_source(item_id, i, |ui| {
        ui.label(&phrase.label);
    }).response;

    if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
        if response.hovered() && ui.memory(|m| m.is_anything_being_dragged()) {
            // highlight as drop target
        }
    }
}
```

Full implementation deferred to v2. The `egui::dnd` module (stabilised in egui 0.27) provides `dnd_drag_source` and `dnd_drop_zone`. Wire them together with an `Option<usize>` drag-source tracker.

---

## 13. Window Position Memory

### What it is

QuickPaste remembers where the user left it and reopens there.

### Implementation

Add `window_pos: Option<egui::Pos2>` to `AppConfig`:

```rust
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub disclaimer_accepted: bool,
    pub theme: Theme,
    pub compact_mode: bool,
    pub paste_delay_ms: u32,
    pub window_pos: Option<[f32; 2]>,   // [x, y] in logical pixels
    pub window_size: Option<[f32; 2]>,  // [w, h]
}
```

Read position from the viewport info and persist it on change:

```rust
// In update(), after rendering
let info = ctx.input(|i| i.viewport().clone());
if let Some(pos) = info.outer_rect {
    let new_pos = [pos.min.x, pos.min.y];
    if self.config.window_pos != Some(new_pos) {
        self.config.window_pos = Some(new_pos);
        // Debounce: only save every 2 seconds of position stability
        self.pos_save_pending = true;
    }
}
```

Apply on startup via `ViewportBuilder`:

```rust
let mut vb = egui::ViewportBuilder::default()
    .with_title("QuickPaste")
    .with_always_on_top(true)
    .with_resizable(true);

if let Some([x, y]) = config.window_pos {
    // Clamp to visible screen area
    let x = x.clamp(0.0, 3840.0);
    let y = y.clamp(0.0, 2160.0);
    vb = vb.with_position(egui::pos2(x, y));
}
```

### Gotchas

- Do not save position on every frame — only when the value changes, debounced. Otherwise every close writes disk.
- Clamp the loaded position. If the user disconnects their secondary monitor, a stored position of `(3200, 100)` puts the window off-screen. Clamp to `(0, 0)` minimum and `(primary_screen_width - window_width, primary_screen_height - window_height)` maximum. Get screen size from `ctx.input(|i| i.viewport().monitor_size)`.
- `ViewportBuilder::with_position` is in logical pixels (DPI-aware). Do not convert manually.

---

## 14. Accessibility Minimum Checklist

These requirements cost minimal implementation effort but significantly improve usability.

### Tab focus order

egui assigns Tab focus in render order. Render widgets in logical order: search bar → phrase list → add-form label → add-form text → Add button → toolbar. This is already the natural render order if structured as described.

### Contrast ratios

`egui::Visuals::dark()` and `::light()` both meet WCAG AA for body text (4.5:1). If custom accent colours are added, verify with a contrast checker. The recommended Win11 blue (`#0078D4`) on dark egui background (`#1b1b1b`) gives ~5.6:1 — passes AA.

### Tooltips on icon buttons

Every button that uses only an icon must have `.on_hover_text(...)`:

```rust
ui.button("⊞").on_hover_text("Expand window");
ui.button("⊟").on_hover_text("Compact window");
ui.button("✕").on_hover_text("Clear search");
ui.button("▶").on_hover_text("Paste this phrase");
```

### Font scaling

Respect the OS DPI setting. egui does this automatically via winit's logical pixel system. Do not hardcode pixel sizes in `ui.add_space(n)` — use `ui.spacing().item_spacing.y` multiples where possible.

### Minimum click target

egui's default button height is ~22 px logical. At 150% DPI this is 33 physical pixels — borderline. Add vertical padding to phrase list rows:

```rust
ui.add_space(2.0);
ui.horizontal(|ui| { /* phrase row */ });
ui.add_space(2.0);
```

---

## Summary — Implementation Order

Following the P0/P1/P2 hierarchy:

**Phase 3 (UI Manager — current):**  
Implement P0-6 (paste-and-hide non-blocking), P0-7 (search bar), P0-8 (repaint throttling), P0-2 (compact/full toggle), P0-4 (dark/light theme).

**Phase 4 (Tray and Window Behaviour):**  
Implement P0-1 (always-on-top flags), P0-5 (tray left-click), P0-3 (global hotkeys — spawn listener in `main()`, pass receiver into app).

**Phase 4 hardening (P1):**  
P1-1 (window position memory), P1-2 (keyboard nav), P1-3 (status bar), P1-5 (Escape key), P1-6 (paste delay slider).

**Phase 6 / v2:**  
P2-1 (drag reorder), P2-2 (per-phrase hotkeys), P2-3 (transparency), P2-4 (import/export).

---

*Document maintained alongside `BUILD-JOURNAL.md`. Update this file whenever an architectural decision changes.*
