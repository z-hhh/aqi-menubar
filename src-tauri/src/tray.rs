use crate::{
    aqicn::Snapshot,
    config::{Config, TrayDisplay},
    icons, AppState,
};
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};
use tauri_plugin_positioner::{Position, WindowExt};

pub const TRAY_ID: &str = "main";
pub const EVENT_OPEN_SETTINGS: &str = "aqi://open-settings";
pub const EVENT_CHECK_UPDATES: &str = "aqi://check-updates";

/// Menu labels: refresh, check for updates, settings, quit.
fn labels(lang: &str) -> [&'static str; 4] {
    match lang {
        "zh-CN" => ["立即刷新", "检查更新…", "设置…", "退出"],
        "ja" => ["今すぐ更新", "アップデートを確認…", "設定…", "終了"],
        _ => ["Refresh now", "Check for updates…", "Settings…", "Quit"],
    }
}

fn build_menu(app: &AppHandle, lang: &str) -> tauri::Result<Menu<tauri::Wry>> {
    let [refresh, updates, settings, quit] = labels(lang);
    let refresh = MenuItem::with_id(app, "refresh", refresh, true, None::<&str>)?;
    let updates = MenuItem::with_id(app, "updates", updates, true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", settings, true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", quit, true, None::<&str>)?;
    Menu::with_items(app, &[&refresh, &updates, &sep1, &settings, &sep2, &quit])
}

pub fn create(app: &AppHandle, cfg: &Config) -> tauri::Result<()> {
    let menu = build_menu(app, &cfg.resolved_language())?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "refresh" => {
                if let Some(state) = app.try_state::<Arc<AppState>>() {
                    state.notify.notify_one();
                }
            }
            "updates" => {
                let _ = app.emit(EVENT_CHECK_UPDATES, ());
            }
            "settings" => {
                show(app);
                let _ = app.emit(EVENT_OPEN_SETTINGS, ());
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            let app = tray.app_handle();
            tauri_plugin_positioner::on_tray_event(app, &event);
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle(app);
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;
    Ok(())
}

/// Rebuild the tray menu after a language change.
pub fn refresh_menu(app: &AppHandle, cfg: &Config) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    if let Ok(menu) = build_menu(app, &cfg.resolved_language()) {
        let _ = tray.set_menu(Some(menu));
    }
}

pub fn show(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.move_window(Position::TrayCenter);
        let _ = win.show();
        let _ = win.set_focus();
    }
}

pub fn toggle(app: &AppHandle) {
    let Some(win) = app.get_webview_window("main") else {
        return;
    };
    if win.is_visible().unwrap_or(false) {
        let _ = win.hide();
    } else {
        show(app);
    }
}

/// What the menu bar shows, given a reading and the user's display settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayText {
    /// The menu bar title (macOS). Empty means "icon only".
    pub title: String,
    /// The hover tooltip / Windows tray tooltip.
    pub tooltip: String,
}

fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let cut: String = s.chars().take(max.saturating_sub(1)).collect();
    format!("{}…", cut.trim_end())
}

fn code_value(snap: &Snapshot, code: &str) -> Option<f64> {
    snap.pollutants
        .iter()
        .find(|p| p.code == code)
        .map(|p| p.value)
        .filter(|v| v.is_finite() && *v >= 0.0)
}

fn aqi_value(snap: &Snapshot) -> Option<i32> {
    snap.aqi.filter(|v| *v >= 0)
}

/// "51" for AQI, "12" for PM2.5; falls back to the AQI when PM2.5 is missing,
/// because a blank menu bar is worse than the headline number.
fn metric_text(snap: &Snapshot, metric: &str) -> Option<(String, &'static str)> {
    match metric {
        "pm25" => match code_value(snap, "pm25") {
            Some(v) => Some(((v.round() as i64).to_string(), "PM2.5")),
            None => aqi_value(snap).map(|v| (v.to_string(), "AQI")),
        },
        "none" => None,
        _ => aqi_value(snap).map(|v| (v.to_string(), "AQI")),
    }
}

pub fn texts(snap: &Snapshot, display: &TrayDisplay) -> TrayText {
    // The title is a menu bar, not a page: keep the station short there and
    // let the tooltip carry the full name.
    let station = truncate_chars(snap.station.trim(), 18);
    let tooltip_station = truncate_chars(snap.station.trim(), 48);
    let pm25 = code_value(snap, "pm25").map(|v| format!("PM2.5 {}", v.round() as i64));
    let aqi = aqi_value(snap);

    // Tooltip: always every useful number, regardless of what the title shows.
    let mut tooltip_parts: Vec<String> = Vec::new();
    tooltip_parts.push(match aqi {
        Some(v) => format!("AQI {v}"),
        None => "AQI –".to_string(),
    });
    if let Some(pm) = pm25 {
        if !pm.ends_with("–") {
            tooltip_parts.push(pm);
        }
    }
    if !tooltip_station.is_empty() {
        tooltip_parts.push(tooltip_station);
    }
    let tooltip = truncate_chars(&tooltip_parts.join(" · "), 120);

    let title = match metric_text(snap, &display.metric) {
        Some((value, label)) => {
            let mut text = if display.show_label {
                format!("{label} {value}")
            } else {
                value
            };
            if display.show_station && !station.is_empty() {
                text.push_str(" · ");
                text.push_str(&station);
            }
            text
        }
        None => match (display.metric.as_str(), aqi) {
            // Icon-only: nothing beside the dot while there is a reading.
            ("none", Some(_)) if display.show_icon => String::new(),
            (_, Some(v)) => v.to_string(),
            // No reading at all: a dash is clearer than a bare grey dot.
            (_, None) => "–".to_string(),
        },
    };

    TrayText { title, tooltip }
}

pub fn update(app: &AppHandle, snap: &Snapshot, display: &TrayDisplay) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let text = texts(snap, display);
    let color = icons::color_for(snap.aqi);

    #[cfg(debug_assertions)]
    eprintln!("tray: title={:?} tooltip={:?}", text.title, text.tooltip);

    #[cfg(target_os = "macos")]
    {
        // An 18 pt dot sits comfortably beside the text in a 22 pt menu bar.
        if display.show_icon {
            let _ = tray.set_icon(Some(icons::dot_image(color, 18)));
        } else {
            let _ = tray.set_icon(None::<tauri::image::Image>);
        }
        let _ = tray.set_title(Some(text.title.as_str()));
        let _ = tray.set_tooltip(Some(text.tooltip.as_str()));
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = tray.set_icon(Some(icons::dot_image(color, 32)));
        let _ = tray.set_tooltip(Some(text.tooltip.as_str()));
    }
}

pub fn clear(app: &AppHandle, display: &TrayDisplay) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    #[cfg(target_os = "macos")]
    {
        if display.show_icon {
            let _ = tray.set_icon(Some(icons::dot_image(
                icons::color_for(None),
                18,
            )));
        } else {
            let _ = tray.set_icon(None::<tauri::image::Image>);
        }
        let _ = tray.set_title(Some("–"));
        let _ = tray.set_tooltip(Some("AQI –"));
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = tray.set_icon(Some(icons::dot_image(
            icons::color_for(None),
            32,
        )));
        let _ = tray.set_tooltip(Some("AQI –"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aqicn::{Pollutant, Snapshot};

    fn snap(aqi: Option<i32>, station: &str, pm25: Option<f64>) -> Snapshot {
        let mut pollutants = Vec::new();
        if let Some(v) = pm25 {
            pollutants.push(Pollutant {
                code: "pm25".into(),
                value: v,
            });
        }
        Snapshot {
            aqi,
            station: station.into(),
            station_url: None,
            dominant: Some("pm25".into()),
            updated: None,
            stale: false,
            pollutants,
            attributions: vec![],
            fetched_at: 0,
        }
    }

    fn display(metric: &str, label: bool, station: bool, icon: bool) -> TrayDisplay {
        TrayDisplay {
            metric: metric.into(),
            show_label: label,
            show_station: station,
            show_icon: icon,
        }
    }

    #[test]
    fn default_is_the_bare_number() {
        let t = texts(&snap(Some(51), "Shanghai (上海)", Some(12.3)), &TrayDisplay::default());
        assert_eq!(t.title, "51");
        assert_eq!(t.tooltip, "AQI 51 · PM2.5 12 · Shanghai (上海)");
    }

    #[test]
    fn label_and_station_are_opt_in() {
        let d = display("aqi", true, true, true);
        let t = texts(&snap(Some(51), "Shanghai (上海)", Some(12.3)), &d);
        assert_eq!(t.title, "AQI 51 · Shanghai (上海)");

        let d = display("aqi", true, false, true);
        assert_eq!(texts(&snap(Some(51), "Shanghai (上海)", None), &d).title, "AQI 51");
    }

    #[test]
    fn pm25_metric_shows_the_particle_number() {
        let d = display("pm25", true, false, true);
        let t = texts(&snap(Some(51), "X", Some(12.6)), &d);
        assert_eq!(t.title, "PM2.5 13");
    }

    #[test]
    fn pm25_metric_falls_back_to_aqi_when_the_station_has_no_pm25() {
        let d = display("pm25", true, false, true);
        assert_eq!(texts(&snap(Some(51), "X", None), &d).title, "AQI 51");
    }

    #[test]
    fn no_metric_keeps_the_dot_only() {
        let d = display("none", true, true, true);
        let t = texts(&snap(Some(51), "Shanghai", Some(12.0)), &d);
        assert_eq!(t.title, "");
        // The tooltip still carries the reading.
        assert!(t.tooltip.starts_with("AQI 51"));
    }

    #[test]
    fn no_metric_without_a_dot_still_shows_the_number() {
        let d = display("none", false, false, false);
        assert_eq!(texts(&snap(Some(51), "X", None), &d).title, "51");
        assert_eq!(texts(&snap(None, "X", None), &d).title, "–");
    }

    #[test]
    fn missing_reading_shows_a_dash() {
        let t = texts(&snap(None, "", None), &TrayDisplay::default());
        assert_eq!(t.title, "–");
        assert_eq!(t.tooltip, "AQI –");
    }

    #[test]
    fn long_station_names_are_truncated_in_both_places() {
        let long = "A".repeat(60);
        let t = texts(&snap(Some(51), &long, None), &display("aqi", false, true, true));
        assert!(t.title.chars().count() <= 24, "{}", t.title);
        assert!(t.title.ends_with('…'));
        assert!(t.tooltip.chars().count() <= 70, "{}", t.tooltip);
    }

    #[test]
    fn no_reading_shows_a_dash_even_with_the_dot() {
        let t = texts(&snap(None, "X", None), &TrayDisplay::default());
        assert_eq!(t.title, "–");
    }
}
