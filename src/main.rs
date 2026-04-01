//! Sokuten — Windows 11 system-tray text expander.
//!
//! Entry point. Initialises logging, enforces single-instance, spawns the
//! global hotkey listener, creates the tray icon, then hands control to the
//! egui application loop.
//!
//! # Safety
//! All unsafe Windows API calls are contained in `inject.rs` and `hotkey.rs`
//! with individual `// SAFETY:` comments.

#![windows_subsystem = "windows"]

#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod hotkey;
mod inject;
mod phrases;
mod tray;
mod ui;

fn main() -> anyhow::Result<()> {
    // Initialise structured logging; respects RUST_LOG env var.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    tracing::info!("Sokuten starting");

    // Enforce single-instance via named mutex.
    let instance =
        single_instance::SingleInstance::new("Sokuten-{7F3A2E1B-4D9C-4B8A-A1E2-3C5F6D7E8F9A}")
            .map_err(|e| anyhow::anyhow!("Failed to check single instance: {e}"))?;

    if !instance.is_single() {
        tracing::warn!("Another instance is already running — exiting");
        return Ok(());
    }

    // Spawn hotkey listener BEFORE eframe::run_native (which blocks).
    let hotkey_rx = hotkey::spawn_hotkey_listener();

    // Create tray icon — must live until main() returns so the icon stays visible.
    let (_tray_icon, tray_menu_ids) = tray::create_tray()?;

    ui::run(hotkey_rx, tray_menu_ids)
}
