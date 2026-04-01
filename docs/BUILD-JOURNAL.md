# Sokuten (速貼) Build Journal

## Session 3 — 2026-03-31 (Migration + Phase 0 audit + Phase 1 + Phase 2)

### Migration: quickpaste → sokuten

| Action | Result |
|---|---|
| New directory created | `D:\Obsidian Vault\PROJECTS\sokuten\` |
| All source files copied + cleaned | `src/`, `docs/`, `Cargo.toml`, `Cargo.lock`, `.gitignore` |
| Assets renamed | `quickpaste.rc/.manifest` → `sokuten.rc/.manifest` |
| `build.rs` updated | References `assets/sokuten.rc` |
| `sokuten.manifest` updated | `name="Sokuten"`, `description="速貼 Sokuten"` |
| `CLAUDE.md` updated | All key paths point to `sokuten\` |
| `settings.json` updated | Added `Write/Edit(sokuten/**)` + `additionalDirectories` |

### Cargo Gates (all run from `D:\Obsidian Vault\PROJECTS\sokuten\`)

| Gate | Status |
|---|---|
| `cargo check` | ✓ PASS |
| `cargo fmt --check` | ✓ PASS |
| `cargo clippy` | ✓ PASS |
| `cargo nextest run` | ✓ PASS — 12/12 |

### Phase 0 — Audit

| Round | Verdict | Findings |
|---|---|---|
| Round 1 | FAIL | F1: thread::sleep in update(), F2: process::exit, F3/F4: doc %APPDATA% mismatch |
| Round 2 | **PASS** | All findings resolved |

**Fixes applied:**
- F1: `thread::sleep` removed — replaced with `PasteState` enum + `poll_paste()` non-blocking state machine
- F2: `std::process::exit(0)` → `ctx.send_viewport_cmd(ViewportCommand::Close)`
- F3/F4: All `%APPDATA%` doc references corrected to `%LOCALAPPDATA%`
- Bonus: `save_phrases` upgraded to atomic write (tmp + rename)

### Phase 1 — phrases.rs test suite

```
cargo nextest run — 8 phrases tests, all PASS
```

| Test | Result |
|---|---|
| `save_and_load_roundtrip` | ✓ |
| `empty_slice_roundtrip` | ✓ |
| `missing_file_returns_empty_vec` | ✓ |
| `malformed_json_is_an_error` | ✓ |
| `config_defaults_to_disclaimer_not_accepted` | ✓ |
| `config_roundtrip_disclaimer_accepted` | ✓ |
| `unicode_cjk_arabic_emoji_roundtrip` | ✓ |
| `large_text_roundtrip` (10,001 chars) | ✓ |

**Phase 1 verdict: PASS**

### Phase 2 — inject.rs test suite

| Test | Result |
|---|---|
| `empty_text_returns_error` | ✓ |
| `make_unicode_input_keydown_flags` | ✓ |
| `make_unicode_input_keyup_flags` | ✓ |
| `make_unicode_input_scan_code` | ✓ |

**Phase 2 verdict: PASS**

### Phase 3 — ui.rs bug fix

- `thread::sleep` removed (F1)
- `process::exit` removed (F2)
- `PasteState` enum introduced
- Audit PASS after fixes

**Phase 3 bug fix: COMPLETE**

---

### Phase 3 (ext) — UI/UX Extensions

**Gates:** check ✓ fmt ✓ clippy ✓ nextest 12/12 ✓

| Feature | Spec | Status |
|---|---|---|
| Always-on-top + resizable | §2 | ✓ `with_always_on_top()`, re-asserted every 10 frames |
| Compact / Full mode toggle | §5 | ✓ `⊞/⊟` toolbar button, persisted |
| Dark / Light theme toggle | §4 | ✓ `☀/☾` toolbar button, persisted, applied via CreationContext |
| Non-blocking paste delay (PasteState) | §10 | ✓ `phrase_idx` + `Instant`, configurable delay |
| Search / filter bar | §7 | ✓ live, case-insensitive, auto-focus on show |
| Idle repaint throttle | §11 | ✓ 100 ms idle, 16 ms during paste wait |
| Window position memory | §13 | ✓ recorded every 30 frames, saved on exit |
| Keyboard nav (↑↓ / Enter / double-click) | §8 | ✓ `selectable_label`, `scroll_to_me` |
| Inline status bar | §9 | ✓ colour-coded, auto-clears after 5 s |
| Escape key hides window | §5 P1-5 | ✓ |
| Paste delay slider | §10 P1-6 | ✓ 50–500 ms |
| `Theme` enum + expanded `AppConfig` | §4 | ✓ serde default for backward compat |
| `ViewportCommand::Close` (F2 fix) | — | ✓ |

**Phase 3: COMPLETE**

---

## Phase 4 — hotkey.rs + tray.rs (Session 4, 2026-04-01)

**Gates:** check ✓ fmt ✓ clippy 0 warnings ✓ nextest 12/12 ✓

### New files

| File | Description |
|---|---|
| `src/hotkey.rs` | `RegisterHotKey`/`UnregisterHotKey` on dedicated OS thread, `GetMessageW` message pump, `mpsc::Sender<i32>` channel to egui thread |
| `src/tray.rs` | `tray-icon 0.19` + `muda 0.15` — tray icon builder, right-click menu (Show/Hide, Toggle Compact, Quit), programmatic 16×16 blue icon |

### Features implemented

| Feature | Spec | Status |
|---|---|---|
| Global hotkey: Ctrl+Shift+Space (show/hide) | §3 P0-3 | ✓ |
| Global hotkey: Ctrl+Shift+V (paste recent) | §3 P0-3 | ✓ |
| RegisterHotKey return value checked + logged | §3 edge cases | ✓ |
| Hotkey thread auto-cleanup (UnregisterHotKey on exit) | §3 | ✓ |
| Tray icon: left-click = toggle visibility | §6 P0-5 | ✓ |
| Tray menu: Show/Hide, Toggle Compact, Quit | §6 P0-5 | ✓ |
| Graceful quit via ViewportCommand::Close | Audit F2 | ✓ |
| Config saved before quit (tray menu) | §6 gotchas | ✓ |
| `paste_most_recent()` for Ctrl+Shift+V | §3 | ✓ |
| Hotkey + tray polling in `update()` (non-blocking) | §3, §6 | ✓ |

### Integration changes

| File | Change |
|---|---|
| `main.rs` | Added `mod hotkey`, `mod tray`; spawn listener + create tray before `eframe::run_native` |
| `ui.rs` | `run()` accepts `Receiver<i32>` + `TrayMenuIds`; `SokutenApp` gains 2 fields; `update()` polls hotkey/tray/menu channels; `toggle_window_visibility` no longer `#[allow(dead_code)]`; new `paste_most_recent()` |

### Compilation fix

| Issue | Fix |
|---|---|
| `TrayIconEvent` is an enum, not a struct with `click_type` | Matched on `TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. }` |
| rustfmt single-line tracing::warn | Auto-fixed by `cargo fmt` |

**Phase 4: COMPLETE**

---

## Phases Remaining

| Phase | Description | Status |
|---|---|---|
| 5 | Release build + binary validation | Pending |
| 6 | GitHub audit + push | Pending |

---

## Previous Sessions

### Session 1+2 — 2026-03-31 (original quickpaste build)

| Check | Result |
|---|---|
| `rustc --version` | `rustc 1.94.1 (e408947bf 2026-03-25)` |
| `cargo --version` | `cargo 1.94.1 (29ea6fb6a 2026-03-24)` |
| Active toolchain | `stable-x86_64-pc-windows-msvc` |
| MSVC linker fix | VS 2022 link.exe pinned in `~/.cargo/config.toml` |
| cargo-nextest | Installed |
| Rename quickpaste → sokuten | Complete |

### Issues resolved in earlier sessions
| Error | Fix |
|---|---|
| `LNK1104: cannot open file 'msvcrt.lib'` | Pinned VS 2022 link.exe |
| `C1083: Cannot open 'vcruntime.h'` | Set CC/CXX/INCLUDE/LIB in [env] |
| `E0432: no KEYBDDATA` | Renamed to KEYBDINPUT |
| `single_instance not found` | Renamed to single-instance (hyphen) |
| `unused CompilationResult` | `let _ = embed_resource::compile(...)` |
| `empty_line_after_doc_comments` | Changed `///` to `//!` in build.rs |
| `dirs::local_data_dir()` not found | Fixed to `dirs::data_local_dir()` |
