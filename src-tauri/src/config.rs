use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// Where the AQI is read from.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Location {
    /// aqicn resolves the location from the caller's IP address.
    #[default]
    Here,
    /// Free text: city name or station name, resolved by aqicn.
    City { query: String },
    /// Specific aqicn station id (the `idx` field).
    Station { uid: u32 },
    /// Explicit coordinates.
    Geo { lat: f64, lon: f64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Config {
    pub token: String,
    pub location: Location,
    pub refresh_minutes: u32,
    /// "auto" | "en" | "zh-CN" | "ja"
    pub language: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            token: String::new(),
            location: Location::Here,
            refresh_minutes: 30,
            language: "auto".to_string(),
        }
    }
}

pub fn detect_language() -> String {
    let locale = sys_locale::get_locale().unwrap_or_else(|| "en".to_string());
    if locale.starts_with("zh") {
        "zh-CN".to_string()
    } else if locale.starts_with("ja") {
        "ja".to_string()
    } else {
        "en".to_string()
    }
}

impl Config {
    pub fn path(app: &AppHandle) -> Option<PathBuf> {
        app.path()
            .app_config_dir()
            .ok()
            .map(|dir| dir.join("config.json"))
    }

    /// Never fails: a missing or unreadable file yields defaults, so the app
    /// always starts and the user can fix settings in the UI.
    pub fn load(app: &AppHandle) -> Self {
        let Some(path) = Self::path(app) else {
            return Self::default();
        };
        let Ok(raw) = std::fs::read_to_string(&path) else {
            return Self::default();
        };
        serde_json::from_str(&raw).unwrap_or_default()
    }

    pub fn save(&self, app: &AppHandle) -> anyhow::Result<()> {
        let path = Self::path(app).ok_or_else(|| anyhow::anyhow!("no config directory"))?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        }
        Ok(())
    }

    /// The language actually used for the UI and the tray menu.
    pub fn resolved_language(&self) -> String {
        if self.language == "auto" {
            detect_language()
        } else {
            self.language.clone()
        }
    }

    pub fn refresh_seconds(&self) -> u64 {
        // aqicn's own stations update hourly; polling faster than 5 min only
        // burns quota, so clamp to that floor.
        (self.refresh_minutes.max(5) as u64) * 60
    }
}
