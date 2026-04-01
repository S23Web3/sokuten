# Sokuten (速貼)

Windows 11 system-tray text expander. Type once, paste many times.

## What it does

Sokuten lets you save text phrases and inject them into any application window with a single click or hotkey. Your clipboard is never touched.

## Install

### Option A — One-click installer (recommended)

1. Download [`install.bat`](https://github.com/S23Web3/sokuten/releases/latest/download/install.bat)
2. Double-click it
3. Done — Sokuten is installed and running

The installer:
- Downloads `sokuten.exe` to `%LOCALAPPDATA%\Sokuten\`
- Creates a Start Menu shortcut
- Optionally adds Sokuten to Windows startup

### Option B — Manual download

1. Go to [Releases](https://github.com/S23Web3/sokuten/releases)
2. Download `sokuten.exe`
3. Put it anywhere you like
4. Run it

### Uninstall

Run `uninstall.bat` from the same Releases page, or just delete `sokuten.exe` and the `%LOCALAPPDATA%\Sokuten\` folder.

## How to use

1. **Launch** — Sokuten appears as a floating window and a tray icon (bottom-right)
2. **Add phrases** — type a label and text, click "Add Phrase"
3. **Paste** — click the **▶** button next to any phrase, or select it and press **Enter**
4. **Hotkeys**:
   - `Ctrl+Shift+Space` — show/hide the window
   - `Ctrl+Shift+V` — instantly paste the first phrase
5. **Tray icon** — left-click to show/hide, right-click for menu

## Features

- Global hotkeys (`Ctrl+Shift+Space`, `Ctrl+Shift+V`)
- System tray with context menu
- Full Unicode support (CJK, Arabic, emoji)
- Dark / Light theme toggle
- Compact mode (phrase list only)
- Live search / filter
- Keyboard navigation (arrow keys, Enter, Escape)
- Configurable paste delay (50-500 ms)
- Window position memory
- Crash-safe file storage
- Single instance enforcement
- No admin privileges required

## Requirements

- Windows 10 or 11
- No Rust, no compiler, no dependencies — just the .exe

## Data storage

Your phrases and settings are saved in `%LOCALAPPDATA%\Sokuten\`:
- `phrases.json` — your saved phrases
- `config.json` — theme, mode, delay, window position

## Build from source (developers only)

```
git clone https://github.com/S23Web3/sokuten.git
cd sokuten
cargo build --release
```

Requires [Rust](https://rustup.rs/) stable toolchain.

## License

MIT
