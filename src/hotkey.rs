//! Global hotkey registration and Win32 message pump.
//!
//! `RegisterHotKey` posts `WM_HOTKEY` to the registering thread's message
//! queue. A dedicated OS thread blocks on `GetMessageW`, then sends the
//! hotkey ID over an `mpsc` channel to the main (egui) thread.
//!
//! # Thread model
//! The listener thread is spawned once at startup and runs until the main
//! thread drops the `Receiver`. On drop, `tx.send()` returns `Err` and the
//! thread exits, calling `UnregisterHotKey` for each registered hotkey.

use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_CONTROL, MOD_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{GetMessageW, MSG, WM_HOTKEY};

/// Hotkey ID: Ctrl+Shift+Space — toggle window visibility.
pub const HOTKEY_SHOW_HIDE: i32 = 1;

/// Hotkey ID: Ctrl+Shift+V — paste the most-recently-used phrase.
pub const HOTKEY_PASTE_RECENT: i32 = 2;

/// Virtual-key code for Space (0x20).
const VK_SPACE: u32 = 0x20;

/// Virtual-key code for V (0x56).
const VK_V: u32 = 0x56;

/// Spawns a dedicated OS thread that registers global hotkeys and pumps
/// `WM_HOTKEY` messages.
///
/// Returns a `Receiver<i32>` that delivers hotkey IDs each time a
/// registered hotkey is pressed. The caller should poll with `try_recv()`
/// inside the egui `update()` loop.
///
/// The thread exits cleanly when the `Receiver` is dropped (main thread
/// exits), unregistering all hotkeys before terminating.
pub fn spawn_hotkey_listener() -> Receiver<i32> {
    let (tx, rx): (Sender<i32>, Receiver<i32>) = mpsc::channel();

    thread::spawn(move || {
        let mods = HOT_KEY_MODIFIERS(MOD_CONTROL.0 | MOD_SHIFT.0);

        // SAFETY: HWND::default() is a null handle — hotkey posts to thread
        // message queue, not a window. RegisterHotKey is safe with valid params.
        unsafe {
            if RegisterHotKey(HWND::default(), HOTKEY_SHOW_HIDE, mods, VK_SPACE).is_err() {
                tracing::warn!(
                    "Failed to register Ctrl+Shift+Space — another app may own this hotkey"
                );
            }

            if RegisterHotKey(HWND::default(), HOTKEY_PASTE_RECENT, mods, VK_V).is_err() {
                tracing::warn!("Failed to register Ctrl+Shift+V — another app may own this hotkey");
            }
        }

        tracing::info!("Hotkey listener started (Ctrl+Shift+Space, Ctrl+Shift+V)");

        let mut msg = MSG::default();
        loop {
            // SAFETY: GetMessageW blocks until a message arrives.
            // HWND::default() retrieves all thread messages.
            // Returns: >0 = message, 0 = WM_QUIT, <0 = error.
            let ret = unsafe { GetMessageW(&mut msg, HWND::default(), 0, 0) };
            if ret.0 <= 0 {
                break; // WM_QUIT or error — exit thread
            }
            if msg.message == WM_HOTKEY {
                let id = msg.wParam.0 as i32;
                if tx.send(id).is_err() {
                    break; // Receiver dropped — main thread exited
                }
            }
        }

        // SAFETY: Unregistering hotkeys we registered above.
        unsafe {
            let _ = UnregisterHotKey(HWND::default(), HOTKEY_SHOW_HIDE);
            let _ = UnregisterHotKey(HWND::default(), HOTKEY_PASTE_RECENT);
        }

        tracing::info!("Hotkey listener thread exiting");
    });

    rx
}
