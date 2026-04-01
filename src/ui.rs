//! UI manager — egui application loop, disclaimer modal, and phrase CRUD.
//!
//! Owns the [`SokutenApp`] state and drives the egui/eframe render loop.
//! Features implemented here:
//!
//! - P0-1: Always-on-top floating window, resizable
//! - P0-2: Compact / Full mode toggle (persisted)
//! - P0-4: Dark / Light theme toggle (persisted)
//! - P0-6: Paste-and-hide via non-blocking [`PasteState`] state machine
//! - P0-7: Live search / filter bar
//! - P0-8: Idle repaint throttle (100 ms)
//! - P1-1: Window position memory
//! - P1-2: Keyboard navigation (↑↓ select, Enter paste, double-click paste)
//! - P1-3: Inline status bar (last action + age, auto-clears after 5 s)
//! - P1-5: Escape key hides window

use crate::hotkey;
use crate::inject;
use crate::phrases::{
    load_config, load_phrases, save_config, save_phrases, AppConfig, Phrase, Theme,
};
use crate::tray::TrayMenuIds;
use anyhow::Result;
use eframe::egui;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

// ── Repaint intervals ────────────────────────────────────────────────────────

/// Idle repaint interval — keeps hotkey/tray polling alive at low CPU cost.
const IDLE_REPAINT_MS: u64 = 100;
/// Fast repaint during paste delay — resolves the delay with minimal latency.
const PASTE_REPAINT_MS: u64 = 16;
/// Re-assert always-on-top every N frames (~1 s at 10 Hz idle).
const TOPMOST_REASSERT_FRAMES: u64 = 10;
/// Clear status bar after this many seconds.
const STATUS_TTL_SECS: u64 = 5;

// ── PasteState ───────────────────────────────────────────────────────────────

/// Non-blocking paste delay state machine.
///
/// Replaces `thread::sleep`. On paste click: window hides, state → `AwaitingFocus`.
/// Each `update()` frame checks elapsed time. Once ≥ `paste_delay_ms` → inject.
enum PasteState {
    /// No paste in progress.
    Idle,
    /// Window hidden; waiting for focus to transfer to the target application.
    AwaitingFocus {
        /// Index into `self.phrases` of the phrase to inject.
        phrase_idx: usize,
        /// When the window-hide command was issued.
        started: Instant,
    },
}

// ── App state ────────────────────────────────────────────────────────────────

/// Top-level egui application state.
pub struct SokutenApp {
    /// All user-saved phrases, loaded once at startup.
    phrases: Vec<Phrase>,
    /// Label field for the "Add phrase" form.
    new_label: String,
    /// Text field for the "Add phrase" form.
    new_text: String,
    /// Whether the first-launch disclaimer has been accepted this session.
    disclaimer_accepted: bool,
    /// Checkbox state on the disclaimer screen.
    do_not_show_again: bool,
    /// Non-blocking paste delay state machine.
    paste_state: PasteState,
    /// Live search / filter query.
    search_query: String,
    /// Index into the *filtered* phrase list that is currently selected (↑↓ nav).
    selected_index: Option<usize>,
    /// Whether the window is in compact (phrase-list-only) mode.
    compact_mode: bool,
    /// Current UI colour theme.
    theme: Theme,
    /// Whether the window is currently visible. Toggled by hotkey / tray / Escape.
    window_visible: bool,
    /// True for one frame after the window becomes visible — auto-focuses search bar.
    just_shown: bool,
    /// Frame counter for periodic always-on-top re-assertion.
    frame_count: u64,
    /// Last action result: `(message, when)`. Green if no "Error" prefix, red otherwise.
    status: Option<(String, Instant)>,
    /// Focus-transfer delay before injection (ms). Loaded from config, editable.
    paste_delay_ms: u32,
    /// Last recorded window position. Updated periodically; saved on exit.
    last_pos: Option<[f32; 2]>,
    /// Receives hotkey IDs from the dedicated hotkey listener thread.
    hotkey_rx: Receiver<i32>,
    /// Menu item IDs for matching tray context-menu events.
    tray_menu_ids: TrayMenuIds,
}

// ── Constructor ──────────────────────────────────────────────────────────────

/// Launches the egui/eframe window and blocks until the app exits.
///
/// Loads config and phrases from disk, applies persisted theme and window
/// position, then hands control to the egui event loop.
///
/// # Errors
/// Returns an error if eframe fails to initialise the native window.
pub fn run(hotkey_rx: Receiver<i32>, tray_menu_ids: TrayMenuIds) -> Result<()> {
    let config = load_config();
    let phrases = load_phrases().unwrap_or_else(|e| {
        tracing::error!("Failed to load phrases: {e} — starting empty");
        Vec::new()
    });

    let theme = config.theme;
    // Force full mode on first run (no phrases) so the add form is visible.
    let compact = if phrases.is_empty() {
        false
    } else {
        config.compact_mode
    };
    let saved_pos = config.window_pos;
    let paste_delay = config.paste_delay_ms;
    let disclaimer = config.disclaimer_accepted;

    let app = SokutenApp {
        phrases,
        new_label: String::new(),
        new_text: String::new(),
        disclaimer_accepted: disclaimer,
        do_not_show_again: false,
        paste_state: PasteState::Idle,
        search_query: String::new(),
        selected_index: None,
        compact_mode: compact,
        theme,
        window_visible: true,
        just_shown: false,
        frame_count: 0,
        status: None,
        paste_delay_ms: paste_delay,
        last_pos: saved_pos,
        hotkey_rx,
        tray_menu_ids,
    };

    let init_size = if compact {
        [360.0_f32, 220.0]
    } else {
        [360.0, 480.0]
    };

    let mut vp = egui::ViewportBuilder::default()
        .with_title("速貼 Sokuten")
        .with_inner_size(init_size)
        .with_min_inner_size([280.0, 180.0])
        .with_resizable(true)
        .with_always_on_top();

    if let Some([x, y]) = saved_pos {
        vp = vp.with_position(egui::pos2(x, y));
    }

    let options = eframe::NativeOptions {
        viewport: vp,
        ..Default::default()
    };

    eframe::run_native(
        "速貼 Sokuten",
        options,
        Box::new(move |cc| {
            // Apply persisted theme before the first frame renders.
            match theme {
                Theme::Dark => cc.egui_ctx.set_visuals(egui::Visuals::dark()),
                Theme::Light => cc.egui_ctx.set_visuals(egui::Visuals::light()),
            }
            Ok(Box::new(app))
        }),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {e}"))
}

// ── eframe::App ──────────────────────────────────────────────────────────────

impl eframe::App for SokutenApp {
    /// Called every frame. Handles paste delay, status expiry, always-on-top
    /// re-assertion, then routes to disclaimer or main UI.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frame_count = self.frame_count.wrapping_add(1);

        // Re-assert always-on-top periodically (another app may have removed it).
        if self.frame_count.is_multiple_of(TOPMOST_REASSERT_FRAMES) {
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                egui::WindowLevel::AlwaysOnTop,
            ));
        }

        // Record window position periodically for position memory.
        if self.frame_count.is_multiple_of(30) {
            if let Some(rect) = ctx.input(|i| i.viewport().inner_rect) {
                self.last_pos = Some([rect.min.x, rect.min.y]);
            }
        }

        // Expire status bar message after TTL.
        if let Some((_, when)) = &self.status {
            if when.elapsed().as_secs() >= STATUS_TTL_SECS {
                self.status = None;
            }
        }

        // Poll non-blocking paste delay.
        self.poll_paste(ctx);

        // Poll global hotkey channel (non-blocking).
        while let Ok(id) = self.hotkey_rx.try_recv() {
            match id {
                hotkey::HOTKEY_SHOW_HIDE => self.toggle_window_visibility(ctx),
                hotkey::HOTKEY_PASTE_RECENT => self.paste_most_recent(ctx),
                _ => {}
            }
        }

        // Poll tray icon click events (left-click up = toggle visibility).
        while let Ok(event) = tray_icon::TrayIconEvent::receiver().try_recv() {
            if matches!(
                event,
                tray_icon::TrayIconEvent::Click {
                    button: tray_icon::MouseButton::Left,
                    button_state: tray_icon::MouseButtonState::Up,
                    ..
                }
            ) {
                self.toggle_window_visibility(ctx);
            }
        }

        // Poll tray context-menu events.
        while let Ok(event) = muda::MenuEvent::receiver().try_recv() {
            if event.id == self.tray_menu_ids.show {
                self.toggle_window_visibility(ctx);
            } else if event.id == self.tray_menu_ids.compact {
                self.toggle_compact(ctx);
            } else if event.id == self.tray_menu_ids.quit {
                if let Err(e) = save_config(&self.current_config()) {
                    tracing::error!("Failed to save config before quit: {e}");
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }

        if !self.disclaimer_accepted {
            self.show_disclaimer(ctx);
        } else {
            self.show_main_ui(ctx);
        }

        // Idle repaint throttle — keeps hotkey/tray polling alive at low CPU cost.
        // Overridden by request_repaint_after(PASTE_REPAINT_MS) during paste wait.
        if !matches!(self.paste_state, PasteState::AwaitingFocus { .. }) {
            ctx.request_repaint_after(Duration::from_millis(IDLE_REPAINT_MS));
        }
    }

    /// Saves config (theme, mode, position) cleanly on window close.
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Err(e) = save_config(&self.current_config()) {
            tracing::error!("Failed to save config on exit: {e}");
        }
    }
}

// ── Core logic ───────────────────────────────────────────────────────────────

impl SokutenApp {
    /// Checks whether the focus-transfer delay has elapsed and fires injection.
    ///
    /// Called at the top of every `update()` frame. Schedules a fast repaint
    /// while waiting so the delay resolves promptly.
    fn poll_paste(&mut self, ctx: &egui::Context) {
        let delay = Duration::from_millis(self.paste_delay_ms as u64);

        let ready = match &self.paste_state {
            PasteState::AwaitingFocus { started, .. } => started.elapsed() >= delay,
            PasteState::Idle => false,
        };

        if ready {
            let phrase_idx = match std::mem::replace(&mut self.paste_state, PasteState::Idle) {
                PasteState::AwaitingFocus { phrase_idx, .. } => phrase_idx,
                PasteState::Idle => unreachable!("guarded by ready flag above"),
            };

            match self.phrases.get(phrase_idx) {
                Some(phrase) => {
                    let label = phrase.label.clone();
                    match inject::send_text(&phrase.text) {
                        Ok(()) => {
                            tracing::info!("Injected phrase \"{}\"", label);
                            self.status = Some((format!("Pasted: {label}"), Instant::now()));
                        }
                        Err(e) => {
                            tracing::error!("Injection failed: {e}");
                            self.status = Some((format!("Error: {e}"), Instant::now()));
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                        }
                    }
                }
                None => {
                    tracing::warn!(
                        "Phrase index {phrase_idx} no longer valid (deleted mid-paste?)"
                    );
                    self.status = Some((
                        "Error: phrase was deleted before paste".into(),
                        Instant::now(),
                    ));
                }
            }
        } else if matches!(self.paste_state, PasteState::AwaitingFocus { .. }) {
            ctx.request_repaint_after(Duration::from_millis(PASTE_REPAINT_MS));
        }
    }

    /// Builds an `AppConfig` snapshot from current app state.
    fn current_config(&self) -> AppConfig {
        AppConfig {
            disclaimer_accepted: self.disclaimer_accepted,
            theme: self.theme,
            compact_mode: self.compact_mode,
            paste_delay_ms: self.paste_delay_ms,
            window_pos: self.last_pos,
        }
    }

    /// Saves the current config snapshot to disk.
    fn persist_config(&self) {
        if let Err(e) = save_config(&self.current_config()) {
            tracing::error!("Failed to persist config: {e}");
        }
    }

    /// Initiates paste: hides window, arms the state machine.
    ///
    /// Actual injection fires in `poll_paste()` after `paste_delay_ms` elapses.
    fn trigger_paste(&mut self, ctx: &egui::Context, phrase_idx: usize) {
        self.status = None;
        self.search_query.clear();
        self.selected_index = None;
        self.paste_state = PasteState::AwaitingFocus {
            phrase_idx,
            started: Instant::now(),
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        self.window_visible = false;
    }

    /// Toggles window visibility. Called by hotkey (Ctrl+Shift+Space) and
    /// tray icon left-click / "Show / Hide" menu item.
    ///
    /// Skipped while a paste is in progress — showing the window during
    /// `AwaitingFocus` would cause `SendInput` to inject text into Sokuten's
    /// own UI instead of the target application.
    fn toggle_window_visibility(&mut self, ctx: &egui::Context) {
        if matches!(self.paste_state, PasteState::AwaitingFocus { .. }) {
            tracing::debug!("Toggle ignored — paste in progress");
            return;
        }
        self.window_visible = !self.window_visible;
        if self.window_visible {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            self.just_shown = true;
        } else {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
    }

    /// Pastes the first phrase in the list via Ctrl+Shift+V hotkey.
    ///
    /// Shows an error in the status bar if the phrase list is empty.
    fn paste_most_recent(&mut self, ctx: &egui::Context) {
        if self.phrases.is_empty() {
            self.status = Some(("Error: no phrases saved".into(), Instant::now()));
            return;
        }
        self.trigger_paste(ctx, 0);
    }

    /// Toggles compact / full mode and resizes the viewport.
    fn toggle_compact(&mut self, ctx: &egui::Context) {
        self.compact_mode = !self.compact_mode;
        let size = if self.compact_mode {
            egui::vec2(360.0, 220.0)
        } else {
            egui::vec2(360.0, 480.0)
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
        self.persist_config();
    }

    /// Returns indices into `self.phrases` that match `search_query`.
    ///
    /// Returns all indices when the query is empty. Matching is case-insensitive
    /// substring on both label and text. Returning plain indices (no references
    /// into `self.phrases`) keeps the borrow checker happy inside `update()`.
    fn filtered_phrase_indices(&self) -> Vec<usize> {
        let q = self.search_query.to_lowercase();
        self.phrases
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                q.is_empty()
                    || p.label.to_lowercase().contains(&q)
                    || p.text.to_lowercase().contains(&q)
            })
            .map(|(i, _)| i)
            .collect()
    }
}

// ── UI panels ────────────────────────────────────────────────────────────────

impl SokutenApp {
    /// Renders the first-launch disclaimer modal.
    ///
    /// Blocks main UI until the user clicks "I Understand" or "Exit".
    fn show_disclaimer(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(16.0);
            ui.heading("速貼 Sokuten \u{2014} Usage Notice");
            ui.add_space(12.0);

            egui::ScrollArea::vertical()
                .max_height(340.0)
                .show(ui, |ui| {
                    ui.label(
                        "速貼 Sokuten injects text you have saved directly into whichever \
                         application window is active on your screen. It uses Windows keyboard \
                         input simulation to do this. Your clipboard is never read or written.\n\
                         \n\
                         By clicking \u{201C}I Understand\u{201D} you acknowledge:\n\
                         \n\
                         \u{2022}  You are solely responsible for the content of phrases you store \
                         and inject.\n\
                         \n\
                         \u{2022}  Do not store passwords, credentials, API keys, or sensitive data.\n\
                         \n\
                         \u{2022}  Injecting text into applications you do not own may violate their \
                         terms of service.\n\
                         \n\
                         \u{2022}  Storing or injecting harmful or illegal content is prohibited.\n\
                         \n\
                         This notice is informational only and does not constitute legal advice.\n\
                         速貼 Sokuten is provided as-is under the MIT licence with no warranty.",
                    );
                });

            ui.add_space(12.0);
            ui.checkbox(&mut self.do_not_show_again, "Do not show this again");
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if ui.button("  I Understand  ").clicked() {
                    self.disclaimer_accepted = true;
                    if self.do_not_show_again {
                        self.persist_config();
                    }
                }
                ui.add_space(16.0);
                if ui.button("  Exit  ").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        });
    }

    /// Renders the main UI: search bar, phrase list, optional add form, toolbar,
    /// and status bar.
    fn show_main_ui(&mut self, ctx: &egui::Context) {
        // Escape key hides the window (P1-5).
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            self.window_visible = false;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            self.show_search_bar(ui);
            ui.separator();
            self.show_phrase_list(ctx, ui);

            if !self.compact_mode {
                ui.separator();
                self.show_add_form(ui);
            }

            ui.separator();
            self.show_toolbar(ctx, ui);
            self.show_status_bar(ui);
        });
    }

    /// Renders the live search / filter bar.
    ///
    /// Clears `selected_index` on query change so keyboard nav stays consistent.
    fn show_search_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("🔍");
            let prev_query = self.search_query.clone();
            let resp = ui.add(
                egui::TextEdit::singleline(&mut self.search_query)
                    .hint_text("Filter phrases…")
                    .desired_width(f32::INFINITY),
            );
            // Auto-focus search bar when the window first becomes visible.
            if self.just_shown {
                resp.request_focus();
                self.just_shown = false;
            }
            if self.search_query != prev_query {
                self.selected_index = None;
            }
            if ui.button("✕").clicked() {
                self.search_query.clear();
                self.selected_index = None;
            }
        });
    }

    /// Renders the scrollable phrase list with keyboard navigation.
    ///
    /// - ↑ / ↓ — move selection
    /// - Enter — paste selected phrase
    /// - Single click — select
    /// - Double-click / [▶] button — paste immediately
    /// - [✕] button — delete phrase
    ///
    /// Shows a quick-start guide when the list is empty.
    fn show_phrase_list(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        // Collect filtered indices FIRST (no lifetime ties to self.phrases),
        // so we can freely mutate self inside the scroll-area closure below.
        let indices = self.filtered_phrase_indices();
        let max_sel = indices.len().saturating_sub(1);

        // ── Keyboard input (read before render loop) ─────────────────────────
        let arrow_down = ui.input(|i| i.key_pressed(egui::Key::ArrowDown));
        let arrow_up = ui.input(|i| i.key_pressed(egui::Key::ArrowUp));
        let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));

        if arrow_down {
            self.selected_index = Some(match self.selected_index {
                None => 0,
                Some(i) => (i + 1).min(max_sel),
            });
        }
        if arrow_up {
            self.selected_index = Some(match self.selected_index {
                None | Some(0) => 0,
                Some(i) => i - 1,
            });
        }

        let mut to_paste: Option<usize> = None;
        let mut to_delete: Option<usize> = None;

        if enter {
            if let Some(sel) = self.selected_index {
                if let Some(&real_idx) = indices.get(sel) {
                    to_paste = Some(real_idx);
                }
            }
        }

        // ── Phrase rows ──────────────────────────────────────────────────────
        // Snapshot state needed inside the closure (avoids conflicting borrows).
        let empty_search = self.search_query.is_empty();
        let current_sel = self.selected_index;
        let mut new_sel = current_sel;

        egui::ScrollArea::vertical().show(ui, |ui| {
            if indices.is_empty() {
                ui.add_space(16.0);
                if empty_search {
                    ui.vertical_centered(|ui| {
                        ui.label("No phrases saved yet.");
                        ui.add_space(12.0);
                        ui.label("─── Quick start ───");
                        ui.add_space(8.0);
                        ui.label("1. Type a Label  (e.g. \"greeting\")");
                        ui.label("2. Type the Text to paste");
                        ui.label("3. Click  Add Phrase");
                        ui.add_space(12.0);
                        ui.label("─── To paste ───");
                        ui.add_space(8.0);
                        ui.label("Click  ▶  next to any phrase");
                        ui.label("or double-click the phrase name");
                        ui.label("or press Enter after selecting one");
                        ui.add_space(12.0);
                        ui.label("─── Hotkeys ───");
                        ui.add_space(8.0);
                        ui.label("Ctrl+Shift+Space  →  show / hide");
                        ui.label("Ctrl+Shift+V       →  paste first phrase");
                        ui.label("Escape             →  hide window");
                    });
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label("No phrases match.");
                    });
                }
            } else {
                for (list_pos, &real_idx) in indices.iter().enumerate() {
                    let selected = new_sel == Some(list_pos);
                    // Clone label so we don't hold a reference into self.phrases.
                    let label = self.phrases[real_idx].label.clone();

                    ui.horizontal(|ui| {
                        let resp = ui.selectable_label(selected, &label);
                        if resp.clicked() {
                            new_sel = Some(list_pos);
                        }
                        if resp.double_clicked() {
                            to_paste = Some(real_idx);
                        }
                        if selected {
                            resp.scroll_to_me(None);
                        }
                        if ui.button("▶").on_hover_text("Paste").clicked() {
                            to_paste = Some(real_idx);
                        }
                        if ui.button("✕").on_hover_text("Delete").clicked() {
                            to_delete = Some(real_idx);
                        }
                    });
                }
            }
        });

        self.selected_index = new_sel;

        // ── Actions (after scroll area — no more borrows on self) ────────────
        if let Some(idx) = to_paste {
            self.trigger_paste(ctx, idx);
        }

        if let Some(idx) = to_delete {
            self.phrases.remove(idx);
            if let Some(sel) = self.selected_index {
                if sel >= self.phrases.len() {
                    self.selected_index = self.phrases.len().checked_sub(1);
                }
            }
            if let Err(e) = save_phrases(&self.phrases) {
                tracing::error!("Failed to save after delete: {e}");
                self.status = Some((format!("Error: save failed: {e}"), Instant::now()));
            } else {
                self.status = Some(("Phrase deleted.".into(), Instant::now()));
            }
        }
    }

    /// Renders the add-phrase form (hidden in compact mode).
    fn show_add_form(&mut self, ui: &mut egui::Ui) {
        ui.label("Label:");
        ui.text_edit_singleline(&mut self.new_label);
        ui.label("Text:");
        ui.text_edit_multiline(&mut self.new_text);

        if ui.button("Add Phrase").clicked() {
            let label = self.new_label.trim().to_string();
            let text = self.new_text.trim().to_string();

            if label.is_empty() || text.is_empty() {
                self.status = Some((
                    "Error: label and text must not be empty.".into(),
                    Instant::now(),
                ));
            } else {
                self.phrases.push(Phrase {
                    label: label.clone(),
                    text,
                });
                self.new_label.clear();
                self.new_text.clear();
                match save_phrases(&self.phrases) {
                    Ok(()) => {
                        self.status = Some((format!("Added: {label}"), Instant::now()));
                    }
                    Err(e) => {
                        tracing::error!("Failed to save phrases: {e}");
                        self.status = Some((format!("Error: save failed: {e}"), Instant::now()));
                    }
                }
            }
        }
    }

    /// Renders the bottom toolbar: compact toggle and theme toggle.
    fn show_toolbar(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Compact / Full mode toggle
            let compact_icon = if self.compact_mode { "⊞" } else { "⊟" };
            let compact_hint = if self.compact_mode {
                "Expand"
            } else {
                "Compact"
            };
            if ui
                .button(compact_icon)
                .on_hover_text(compact_hint)
                .clicked()
            {
                self.toggle_compact(ctx);
            }

            ui.add_space(8.0);

            // Dark / Light theme toggle
            let theme_label = match self.theme {
                Theme::Dark => "☀ Light",
                Theme::Light => "☾ Dark",
            };
            if ui.button(theme_label).clicked() {
                self.theme = match self.theme {
                    Theme::Dark => Theme::Light,
                    Theme::Light => Theme::Dark,
                };
                match self.theme {
                    Theme::Dark => ctx.set_visuals(egui::Visuals::dark()),
                    Theme::Light => ctx.set_visuals(egui::Visuals::light()),
                }
                self.persist_config();
            }

            ui.add_space(8.0);

            // Paste delay slider (P1-6)
            ui.label("Delay:");
            ui.add(
                egui::Slider::new(&mut self.paste_delay_ms, 50..=500)
                    .text("ms")
                    .step_by(10.0),
            );
        });
    }

    /// Renders the inline status bar.
    ///
    /// Green for success messages, red for messages starting with "Error".
    /// Shows elapsed seconds since the last action.
    fn show_status_bar(&self, ui: &mut egui::Ui) {
        if let Some((ref msg, when)) = self.status {
            let age = when.elapsed().as_secs();
            let colour = if msg.starts_with("Error") {
                egui::Color32::from_rgb(255, 100, 100)
            } else {
                egui::Color32::from_rgb(100, 210, 100)
            };
            ui.horizontal(|ui| {
                ui.colored_label(colour, msg);
                ui.label(format!("({age}s ago)"));
            });
        }
    }
}
