# Sokuten (速貼)

Windows 11 system-tray text expander. Type once, paste many times.

## What it does

Sokuten lets you save text phrases and inject them into any application window with a single click or hotkey. It uses Windows `SendInput` with `KEYEVENTF_UNICODE` — your clipboard is never touched.

## Features

- **System tray** — lives in the notification area, left-click to show/hide
- **Global hotkeys** — `Ctrl+Shift+Space` toggle window, `Ctrl+Shift+V` paste most recent
- **Unicode injection** — full Unicode support including CJK, Arabic, emoji (surrogate pairs)
- **Dark / Light theme** — toggle with one click, persisted across restarts
- **Compact mode** — phrase list only, or expand to show the add-phrase form
- **Live search** — filter phrases by label or text, case-insensitive
- **Keyboard navigation** — arrow keys to select, Enter to paste, Escape to hide
- **Non-blocking paste** — configurable delay (50-500 ms) via state machine, no UI freeze
- **Window position memory** — reopens where you left it
- **Atomic persistence** — phrases and config saved via tmp+rename, crash-safe
- **Single instance** — only one copy runs at a time

## Requirements

- Windows 11 (or Windows 10 with recent updates)
- No admin privileges required

## Build

```bash
rustup default stable
cargo build --release
```

Binary: `target/release/sokuten.exe` (~4.6 MB)

## Stack

| Component | Crate | Version |
|-----------|-------|---------|
| GUI | egui + eframe | 0.29 |
| Windows API | windows | 0.58 |
| System tray | tray-icon | 0.19 |
| Tray menu | muda | 0.15 |
| Persistence | serde + serde_json | 1 |
| Error handling | anyhow | 1 |
| Logging | tracing | 0.1 |
| Allocator | mimalloc | 0.1 |

## Data storage

Phrases and config are stored in `%LOCALAPPDATA%\Sokuten\`:
- `phrases.json` — saved phrases
- `config.json` — theme, mode, delay, window position, disclaimer state

## License

MIT
