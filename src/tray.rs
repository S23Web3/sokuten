//! System tray icon and context menu.
//!
//! Creates a notification-area icon with a right-click context menu using
//! `tray-icon 0.19` + `muda 0.15`. The icon persists for the lifetime of
//! the returned [`tray_icon::TrayIcon`] — dropping it removes the icon.
//!
//! Events are polled via `muda::MenuEvent::receiver()` and
//! `tray_icon::TrayIconEvent::receiver()` inside the egui `update()` loop.

use anyhow::Result;
use muda::{Menu, MenuItem, PredefinedMenuItem};
use tray_icon::TrayIconBuilder;

/// Menu item IDs stored in [`crate::ui::SokutenApp`] for event matching.
pub struct TrayMenuIds {
    /// "Show / Hide" menu item.
    pub show: muda::MenuId,
    /// "Toggle Compact" menu item.
    pub compact: muda::MenuId,
    /// "Quit" menu item.
    pub quit: muda::MenuId,
}

/// Creates the system tray icon and context menu.
///
/// Returns the `TrayIcon` handle (must be kept alive for the icon to
/// remain visible) and the menu item IDs for event matching in `update()`.
///
/// # Errors
/// Returns an error if the tray icon or menu cannot be created.
pub fn create_tray() -> Result<(tray_icon::TrayIcon, TrayMenuIds)> {
    let menu = Menu::new();

    let show_item = MenuItem::new("Show / Hide", true, None);
    let compact_item = MenuItem::new("Toggle Compact", true, None);
    let quit_item = MenuItem::new("Quit", true, None);

    let sep = PredefinedMenuItem::separator();
    menu.append_items(&[&show_item, &compact_item, &sep, &quit_item])?;

    let icon = create_default_icon();

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("速貼 Sokuten")
        .with_icon(icon)
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build tray icon: {e}"))?;

    let ids = TrayMenuIds {
        show: show_item.id().clone(),
        compact: compact_item.id().clone(),
        quit: quit_item.id().clone(),
    };

    tracing::info!("Tray icon created");
    Ok((tray, ids))
}

/// Generates a simple 16x16 blue-square icon for the notification area.
///
/// This is a placeholder; a proper `.ico` asset can be substituted later
/// via `Icon::from_rgba` with the real image data.
fn create_default_icon() -> tray_icon::Icon {
    let size = 16u32;
    let mut rgba = vec![0u8; (size * size * 4) as usize];

    for y in 0..size {
        for x in 0..size {
            let idx = ((y * size + x) * 4) as usize;
            if x == 0 || x == size - 1 || y == 0 || y == size - 1 {
                // White border
                rgba[idx] = 255;
                rgba[idx + 1] = 255;
                rgba[idx + 2] = 255;
                rgba[idx + 3] = 255;
            } else {
                // Windows 11 blue accent fill
                rgba[idx] = 0;
                rgba[idx + 1] = 120;
                rgba[idx + 2] = 212;
                rgba[idx + 3] = 255;
            }
        }
    }

    tray_icon::Icon::from_rgba(rgba, size, size).expect("programmatic icon data is always valid")
}
