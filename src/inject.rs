//! Text injection core — sends phrases to the focused window via `SendInput`.
//!
//! Uses `KEYEVENTF_UNICODE` to inject Unicode characters including surrogate
//! pairs (emoji, non-BMP). The clipboard is never read or written.
//!
//! # Security
//! - Only plain Unicode key events are sent; no virtual-key combos.
//! - No admin escalation key sequences (Win, Ctrl+Alt+Del, etc.) are generated.
//! - Injection is gated by an explicit UI action — no autonomous triggering.

use anyhow::{bail, Result};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
    VIRTUAL_KEY,
};

/// Injects `text` into the currently focused application window using
/// Windows `SendInput` with `KEYEVENTF_UNICODE`.
///
/// Each Unicode scalar value is decomposed into UTF-16 code units. Characters
/// outside the Basic Multilingual Plane (emoji, etc.) are sent as a surrogate
/// pair — one keydown + keyup per code unit.
///
/// # Errors
/// Returns an error if:
/// - `text` is empty (no-op guard)
/// - `SendInput` reports that fewer events were accepted than sent (system
///   blocked the input, e.g. a UAC / privilege boundary was hit)
///
/// # Security
/// Injection is only allowed when explicitly triggered by the user via a
/// UI button press. This function has no timer or background path.
pub fn send_text(text: &str) -> Result<()> {
    if text.is_empty() {
        bail!("send_text called with empty string — nothing to inject");
    }

    // Build INPUT events: keydown + keyup for every UTF-16 code unit.
    let mut events: Vec<INPUT> = Vec::new();

    for code_unit in text.encode_utf16() {
        events.push(make_unicode_input(code_unit, false));
        events.push(make_unicode_input(code_unit, true));
    }

    let n = events.len() as u32;

    // SAFETY: `events` is a valid slice of correctly initialised INPUT
    // structs. `INPUT` is `#[repr(C)]` and all fields are initialised.
    // `SendInput` is documented to accept a pointer + count + struct size;
    // we pass exactly those. The call does not mutate the slice.
    let sent = unsafe { SendInput(&events, std::mem::size_of::<INPUT>() as i32) };

    if sent != n {
        bail!(
            "SendInput accepted {sent}/{n} events — \
             input may have been blocked by a privileged window"
        );
    }

    tracing::debug!("Injected {} UTF-16 code units ({} events)", n / 2, n);
    Ok(())
}

/// Constructs a single `INPUT` struct for a Unicode key event.
///
/// - `code_unit`: a UTF-16 code unit (may be a surrogate half for non-BMP chars)
/// - `key_up`: `false` for keydown, `true` for keyup
///
/// The `wVk` (virtual-key code) field is intentionally left at 0 — the
/// system uses the `wScan` (Unicode) field when `KEYEVENTF_UNICODE` is set.
fn make_unicode_input(code_unit: u16, key_up: bool) -> INPUT {
    let mut flags = KEYEVENTF_UNICODE;
    if key_up {
        flags |= KEYEVENTF_KEYUP;
    }

    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(0),
                wScan: code_unit,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_text_returns_error() {
        let result = send_text("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty string"));
    }

    #[test]
    fn make_unicode_input_keydown_flags() {
        let input = make_unicode_input(0x0041, false); // 'A'
                                                       // SAFETY: We constructed this INPUT ourselves and know it is KEYBOARD type.
        let flags = unsafe { input.Anonymous.ki.dwFlags };
        assert_eq!(flags, KEYEVENTF_UNICODE);
    }

    #[test]
    fn make_unicode_input_keyup_flags() {
        let input = make_unicode_input(0x0041, true); // 'A' keyup
                                                      // SAFETY: We constructed this INPUT ourselves and know it is KEYBOARD type.
        let flags = unsafe { input.Anonymous.ki.dwFlags };
        assert_eq!(flags, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP);
    }

    #[test]
    fn make_unicode_input_scan_code() {
        let input = make_unicode_input(0x4E2D, false); // CJK '中'
                                                       // SAFETY: We constructed this INPUT ourselves and know it is KEYBOARD type.
        let scan = unsafe { input.Anonymous.ki.wScan };
        assert_eq!(scan, 0x4E2D);
    }
}
