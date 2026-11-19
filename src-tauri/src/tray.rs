use crate::{aqicn::Snapshot, config::Config, AppState};
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};
use tauri_plugin_positioner::{Position, WindowExt};

pub const TRAY_ID: &str = "main";
pub const EVENT_OPEN_SETTINGS: &str = "aqi://open-settings";

fn labels(lang: &str) -> [&'static str; 3] {
    match lang {
        "zh-CN" => ["立即刷新", "设置…", "退出"],
        "ja" => ["今すぐ更新", "設定…", "終了"],
        _ => ["Refresh now", "Settings…", "Quit"],
    }
}

fn build_menu(app: &AppHandle, lang: &str) -> tauri::Result<Menu<tauri::Wry>> {
    let [refresh, settings, quit] = labels(lang);
    let refresh = MenuItem::with_id(app, "refresh", refresh, true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", settings, true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", quit, true, None::<&str>)?;
    Menu::with_items(app, &[&refresh, &settings, &sep, &quit])
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

fn tooltip(snap: &Snapshot) -> String {
    let value = match snap.aqi {
        Some(v) if v >= 0 => v.to_string(),
        _ => "–".to_string(),
    };
    let station: String = snap.station.chars().take(48).collect();
    if station.is_empty() {
        format!("AQI {value}")
    } else {
        format!("AQI {value} · {station}")
    }
}

pub fn update(app: &AppHandle, snap: &Snapshot) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let text = match snap.aqi {
        Some(v) if v >= 0 => v.to_string(),
        _ => "–".to_string(),
    };
    let tip = tooltip(snap);

    #[cfg(debug_assertions)]
    eprintln!("tray: {}", tip);

    #[cfg(target_os = "macos")]
    {
        // A text-only status item is the idiomatic menu bar presentation.
        let _ = tray.set_icon(None::<tauri::image::Image>);
        let _ = tray.set_title(Some(text.as_str()));
        let _ = tray.set_tooltip(Some(tip.as_str()));
    }

    #[cfg(not(target_os = "macos"))]
    {
        let color = icons::color_for(snap.aqi);
        let _ = tray.set_icon(Some(crate::icons::dot_image(color, 32)));
        let _ = tray.set_tooltip(Some(tip.as_str()));
    }
}

pub fn clear(app: &AppHandle) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    #[cfg(target_os = "macos")]
    {
        let _ = tray.set_icon(None::<tauri::image::Image>);
        let _ = tray.set_title(Some("–"));
        let _ = tray.set_tooltip(Some("AQI –"));
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = tray.set_icon(Some(crate::icons::dot_image(
            crate::icons::color_for(None),
            32,
        )));
        let _ = tray.set_tooltip(Some("AQI –"));
    }
}
