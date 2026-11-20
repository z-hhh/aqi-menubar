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

/// What the menu bar / tray shows. See `tray::texts` for the composition rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TrayDisplay {
    /// Which number to show: "aqi" | "pm25" | "none".
    /// `none` keeps only the colour dot, for people who want an icon and
    /// nothing else in the menu bar.
    pub metric: String,
    /// Prefix the number with its name: "AQI 51" rather than "51".
    pub show_label: bool,
    /// Append the quality level: "51 · 良". Uses the short word for the UI
    /// language, because a menu bar cannot hold "Unhealthy for sensitive groups".
    pub show_category: bool,
    /// Append the station name: "51 · Shanghai (上海)".
    pub show_station: bool,
    /// A small EPA-coloured dot before the text (macOS). Windows cannot show
    /// text in the notification area at all, so there the dot is the icon and
    /// this flag is ignored.
    pub show_icon: bool,
    /// Draw the menu bar text itself in the level colour (macOS). Off by
    /// default: the plain label colour is what every other menu bar app uses,
    /// and purple-on-grey is not always legible.
    pub color_text: bool,
}

impl Default for TrayDisplay {
    fn default() -> Self {
        Self {
            metric: "aqi".to_string(),
            show_label: false,
            show_category: true,
            show_station: false,
            show_icon: true,
            color_text: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Config {
    pub token: String,
    pub location: Location,
    pub refresh_minutes: u32,
    /// "auto" | "en" | "zh-CN" | "ja"
    pub language: String,
    pub tray: TrayDisplay,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            token: String::new(),
            location: Location::Here,
            refresh_minutes: 30,
            language: "auto".to_string(),
            tray: TrayDisplay::default(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn older_configs_without_a_tray_block_get_the_defaults() {
        let json = r#"{"token":"abc","location":{"kind":"here"},"refreshMinutes":30,"language":"en"}"#;
        let cfg: Config = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.tray.metric, "aqi");
        assert!(cfg.tray.show_icon);
        // The level word is on by default: the number alone says little.
        assert!(cfg.tray.show_category);
        assert!(!cfg.tray.show_station);
        assert_eq!(cfg.token, "abc");
    }

    #[test]
    fn tray_display_round_trips_as_camel_case() {
        let cfg = Config {
            tray: TrayDisplay {
                metric: "pm25".into(),
                show_label: true,
                show_category: false,
                show_station: true,
                show_icon: false,
                color_text: true,
            },
            ..Config::default()
        };
        let json = serde_json::to_string(&cfg).unwrap();
        assert!(json.contains("\"showLabel\":true"), "{json}");
        assert!(json.contains("\"showCategory\":false"), "{json}");
        assert!(json.contains("\"showStation\":true"), "{json}");
        let back: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(back.tray.metric, "pm25");
        assert!(!back.tray.show_icon);
        assert!(!back.tray.show_category);
    }
}
