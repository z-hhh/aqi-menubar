mod aqicn;
mod commands;
mod config;
#[cfg_attr(target_os = "macos", allow(dead_code))]
mod icons;
mod level;
mod tray;

use commands::UpdateInfo;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

pub const EVENT_UPDATE: &str = "aqi://update";

pub struct AppState {
    pub config: Mutex<config::Config>,
    pub snapshot: Mutex<Option<aqicn::Snapshot>>,
    pub error: Mutex<Option<String>>,
    /// The newest release the updater knows about, if any.
    pub update: Mutex<Option<UpdateInfo>>,
    /// Set while an install is running, so the UI can show progress.
    pub installing: Mutex<bool>,
    /// Woken whenever settings change or the user asks for a refresh.
    pub notify: tokio::sync::Notify,
}

impl AppState {
    fn new(config: config::Config) -> Self {
        Self {
            config: Mutex::new(config),
            snapshot: Mutex::new(None),
            error: Mutex::new(None),
            update: Mutex::new(None),
            installing: Mutex::new(false),
            notify: tokio::sync::Notify::new(),
        }
    }
}

async fn poll_loop(app: AppHandle, state: Arc<AppState>) {
    loop {
        let cfg = state.config.lock().unwrap().clone();

        if cfg.token.trim().is_empty() {
            *state.error.lock().unwrap() = Some("no-token".to_string());
            tray::clear(&app, &cfg.tray, &cfg.resolved_language());
            commands::emit_state(&app, &state);
        } else {
            match aqicn::fetch(&cfg).await {
                Ok(snapshot) => {
                    *state.snapshot.lock().unwrap() = Some(snapshot.clone());
                    *state.error.lock().unwrap() = None;
                    tray::update(&app, &snapshot, &cfg.tray, &cfg.resolved_language());
                }
                Err(err) => {
                    #[cfg(debug_assertions)]
                    eprintln!("poll: fetch failed: {err}");
                    // Keep the last reading visible but mark it stale.
                    if let Some(last) = state.snapshot.lock().unwrap().as_mut() {
                        last.stale = true;
                    }
                    *state.error.lock().unwrap() = Some(err.to_string());
                }
            }
            commands::emit_state(&app, &state);
        }

        let wait = std::time::Duration::from_secs(cfg.refresh_seconds());
        tokio::select! {
            _ = tokio::time::sleep(wait) => {}
            _ = state.notify.notified() => {}
        }
    }
}

/// Check the GitHub release feed once, remember the result and tell the UI.
pub async fn check_for_update(app: &AppHandle, notify_ui: bool) -> Option<UpdateInfo> {
    let state = app.try_state::<Arc<AppState>>()?;
    let updater = match app.updater() {
        Ok(updater) => updater,
        Err(err) => {
            eprintln!("updater unavailable: {err}");
            return None;
        }
    };

    let found = match updater.check().await {
        Ok(Some(update)) => Some(UpdateInfo {
            version: update.version.clone(),
            current_version: update.current_version.clone(),
            notes: update.body.clone(),
        }),
        Ok(None) => None,
        Err(err) => {
            eprintln!("update check failed: {err}");
            return None;
        }
    };

    *state.update.lock().unwrap() = found.clone();
    if notify_ui {
        let _ = app.emit(EVENT_UPDATE, found.clone());
    }
    found
}

/// Look for a new release shortly after launch, then every six hours.
async fn update_loop(app: AppHandle) {
    tokio::time::sleep(std::time::Duration::from_secs(20)).await;
    loop {
        check_for_update(&app, true).await;
        tokio::time::sleep(std::time::Duration::from_secs(6 * 3600)).await;
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .on_window_event(|window, event| {
            // A menu bar popover closes when it loses focus.
            // AQI_MENUBAR_PIN keeps it open for screenshots and UI checks.
            if let tauri::WindowEvent::Focused(false) = event {
                if window.label() == "main" && std::env::var("AQI_MENUBAR_PIN").is_err() {
                    let _ = window.hide();
                }
            }
        })
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let cfg = config::Config::load(app.handle());
            let state = Arc::new(AppState::new(cfg.clone()));
            app.manage(state.clone());

            tray::create(app.handle(), &cfg)?;
            tray::clear(app.handle(), &cfg.tray, &cfg.resolved_language());

            // First run: open the panel so the token step is obvious.
            // AQI_MENUBAR_SHOW / AQI_MENUBAR_DEV_VIEW are developer hooks for
            // screenshots and e2e checks ("settings" opens straight on Settings).
            let dev_view = std::env::var("AQI_MENUBAR_DEV_VIEW").ok();
            if cfg.token.trim().is_empty()
                || std::env::var("AQI_MENUBAR_SHOW").is_ok()
                || dev_view.is_some()
            {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    tray::show(&handle);
                    if dev_view.as_deref() == Some("settings") {
                        let _ = handle.emit(tray::EVENT_OPEN_SETTINGS, ());
                    }
                });
            }

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                poll_loop(handle, state).await;
            });

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                update_loop(handle).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::save_config,
            commands::refresh_now,
            commands::search_stations,
            commands::preview_tray,
            commands::check_update,
            commands::install_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running AQI Menubar");
}
