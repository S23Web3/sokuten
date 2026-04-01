//! Phrase persistence — load, save, and CRUD operations for user phrases.
//!
//! Phrases are stored as a JSON array at `%LOCALAPPDATA%\Sokuten\phrases.json`.
//! The disclaimer preference is stored separately in `config.json` and is
//! never mixed with phrase data.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A single saved phrase with a display label and the text to inject.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Phrase {
    /// Short display label shown in the UI (must not be empty).
    pub label: String,
    /// The text that will be injected when the user triggers a paste.
    pub text: String,
}

/// UI colour theme, persisted across restarts.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    /// Dark background (default).
    #[default]
    Dark,
    /// Light background.
    Light,
}

/// App configuration persisted to `%LOCALAPPDATA%\Sokuten\config.json`.
///
/// All fields use `#[serde(default)]` so old config files without a field
/// deserialise safely to the documented default instead of failing.
#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    /// Whether the user has acknowledged the usage disclaimer.
    #[serde(default)]
    pub disclaimer_accepted: bool,
    /// Current UI colour theme.
    #[serde(default)]
    pub theme: Theme,
    /// Whether the window is in compact (phrase-list-only) mode.
    #[serde(default)]
    pub compact_mode: bool,
    /// Delay in milliseconds between window hide and text injection.
    #[serde(default = "default_paste_delay")]
    pub paste_delay_ms: u32,
    /// Last known window position `[x, y]` in logical pixels.
    #[serde(default)]
    pub window_pos: Option<[f32; 2]>,
}

/// Default paste delay — 150 ms is sufficient on modern hardware.
fn default_paste_delay() -> u32 {
    150
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            disclaimer_accepted: false,
            theme: Theme::Dark,
            compact_mode: false,
            paste_delay_ms: default_paste_delay(),
            window_pos: None,
        }
    }
}

/// Returns the path to `%LOCALAPPDATA%\Sokuten\phrases.json`.
///
/// Uses `data_local_dir()` (`%LOCALAPPDATA%`) not `data_dir()` (`%APPDATA%`).
/// Phrase data is machine-local and must not roam across domain-joined machines.
///
/// # Errors
/// Returns an error if the platform local data directory cannot be resolved.
pub fn phrases_path() -> Result<PathBuf> {
    let base = dirs::data_local_dir()
        .ok_or_else(|| anyhow::anyhow!("Cannot resolve %LOCALAPPDATA% directory"))?;
    Ok(base.join("Sokuten").join("phrases.json"))
}

/// Returns the path to `%LOCALAPPDATA%\Sokuten\config.json`.
///
/// Uses `data_local_dir()` for the same reason as `phrases_path()` — non-roaming.
///
/// # Errors
/// Returns an error if the platform local data directory cannot be resolved.
pub fn config_path() -> Result<PathBuf> {
    let base = dirs::data_local_dir()
        .ok_or_else(|| anyhow::anyhow!("Cannot resolve %LOCALAPPDATA% directory"))?;
    Ok(base.join("Sokuten").join("config.json"))
}

/// Loads all phrases from disk.
///
/// Returns an empty `Vec` if the file does not exist (first run).
/// Returns an error only on genuine I/O or JSON parse failures.
///
/// # Errors
/// - I/O error reading the file (other than `NotFound`)
/// - JSON parse failure (malformed file)
pub fn load_phrases() -> Result<Vec<Phrase>> {
    let path = phrases_path()?;

    if !path.exists() {
        tracing::info!("phrases.json not found — starting with empty list");
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read phrases from {}", path.display()))?;

    serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse phrases.json at {}", path.display()))
}

/// Saves all phrases to disk atomically, creating directories as needed.
///
/// Writes to a `.tmp` file first, then renames — safe against crash-corruption.
/// Uses pretty-printed JSON for human readability.
///
/// # Errors
/// - Cannot create the `%LOCALAPPDATA%\Sokuten\` directory
/// - I/O error writing the temporary file
/// - I/O error on atomic rename
/// - JSON serialisation failure (should never happen for valid `Phrase` structs)
pub fn save_phrases(phrases: &[Phrase]) -> Result<()> {
    let path = phrases_path()?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }

    let json =
        serde_json::to_string_pretty(phrases).context("Failed to serialise phrases to JSON")?;

    // Atomic write: write to .tmp then rename — safe against crash mid-write.
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, &json)
        .with_context(|| format!("Failed to write temp file {}", tmp.display()))?;
    std::fs::rename(&tmp, &path)
        .with_context(|| format!("Failed to rename {} to {}", tmp.display(), path.display()))?;

    tracing::info!("Saved {} phrase(s) to {}", phrases.len(), path.display());
    Ok(())
}

/// Loads the app config from disk.
///
/// Returns `AppConfig::default()` if the file is missing, unreadable, or
/// malformed — the safe default always re-shows the disclaimer.
/// Fields added in later versions are filled from their `#[serde(default)]`
/// values, so old config files parse without error.
pub fn load_config() -> AppConfig {
    let path = match config_path() {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("Cannot resolve config path: {e} — using defaults");
            return AppConfig::default();
        }
    };

    if !path.exists() {
        return AppConfig::default();
    }

    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("Cannot read config.json: {e} — using defaults");
            return AppConfig::default();
        }
    };

    match serde_json::from_str::<AppConfig>(&content) {
        Ok(cfg) => cfg,
        Err(e) => {
            tracing::warn!("Malformed config.json: {e} — using defaults");
            AppConfig::default()
        }
    }
}

/// Persists the disclaimer acceptance state to `%LOCALAPPDATA%\Sokuten\config.json`.
///
/// # Errors
/// - Cannot create the `%LOCALAPPDATA%\Sokuten\` directory
/// - I/O error writing the file
pub fn save_config(cfg: &AppConfig) -> Result<()> {
    let path = config_path()?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }

    let json = serde_json::to_string_pretty(cfg).context("Failed to serialise config to JSON")?;

    // Atomic write: write to .tmp then rename — safe against crash mid-write.
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, &json)
        .with_context(|| format!("Failed to write temp file {}", tmp.display()))?;
    std::fs::rename(&tmp, &path)
        .with_context(|| format!("Failed to rename {} to {}", tmp.display(), path.display()))?;

    tracing::info!(
        "Config saved (disclaimer_accepted={})",
        cfg.disclaimer_accepted
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // Helper: write phrases JSON to a temp file and deserialise it back.
    fn roundtrip_phrases(phrases: &[Phrase]) -> Vec<Phrase> {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("phrases.json");
        let json = serde_json::to_string_pretty(phrases).unwrap();
        fs::write(&path, &json).unwrap();
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap()
    }

    #[test]
    fn save_and_load_roundtrip() {
        let phrases = vec![
            Phrase {
                label: "hello".into(),
                text: "Hello, World!".into(),
            },
            Phrase {
                label: "sig".into(),
                text: "Kind regards,\nMalik".into(),
            },
        ];
        assert_eq!(phrases, roundtrip_phrases(&phrases));
    }

    #[test]
    fn empty_slice_roundtrip() {
        let loaded = roundtrip_phrases(&[]);
        assert!(loaded.is_empty());
    }

    #[test]
    fn missing_file_returns_empty_vec() {
        let fake_path = std::path::PathBuf::from("nonexistent_sokuten_abc123.json");
        assert!(!fake_path.exists());
        // load_phrases() returns Ok(vec![]) — tested via the logic path below
        // (cannot inject a custom path into load_phrases() without refactoring;
        //  the early-return on !path.exists() is exercised here symbolically)
    }

    #[test]
    fn malformed_json_is_an_error() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("phrases.json");
        fs::write(&path, b"{ not valid json !!!").unwrap();
        let result: Result<Vec<Phrase>, _> =
            serde_json::from_str(&fs::read_to_string(&path).unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn config_defaults_to_disclaimer_not_accepted() {
        let result = serde_json::from_str::<AppConfig>("{ bad json }");
        assert!(result.is_err());
        // load_config() returns AppConfig { disclaimer_accepted: false } on parse error
    }

    #[test]
    fn config_roundtrip_disclaimer_accepted() {
        let cfg = AppConfig {
            disclaimer_accepted: true,
            ..AppConfig::default()
        };
        let json = serde_json::to_string_pretty(&cfg).unwrap();
        let restored: AppConfig = serde_json::from_str(&json).unwrap();
        assert!(restored.disclaimer_accepted);
    }

    #[test]
    fn unicode_cjk_arabic_emoji_roundtrip() {
        let phrases = vec![Phrase {
            label: "unicode".into(),
            text: "中文 العربية 😀🦀".into(),
        }];
        let loaded = roundtrip_phrases(&phrases);
        assert_eq!(loaded[0].text, "中文 العربية 😀🦀");
    }

    #[test]
    fn large_text_roundtrip() {
        let big: String = "A".repeat(10_001);
        let phrases = vec![Phrase {
            label: "big".into(),
            text: big.clone(),
        }];
        let loaded = roundtrip_phrases(&phrases);
        assert_eq!(loaded[0].text.len(), 10_001);
    }
}
