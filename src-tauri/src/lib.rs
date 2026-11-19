mod aqicn;
mod commands;
mod config;
#[cfg_attr(target_os = "macos", allow(dead_code))]
mod icons;
mod tray;

use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};

pub struct AppState {
    pub config: Mutex<config::Config>,
    pub snapshot: Mutex<Option<aqicn::Snapshot>>,
    pub error: Mutex<Option<String>>,
    /// Woken whenever settings change or the user asks for a refresh.
    pub notify: tokio::sync::Notify,
}

impl AppState {
    fn new(config: config::Config) -> Self {
        Self {
            config: Mutex::new(config),
            snapshot: Mutex::new(None),
            error: Mutex::new(None),
            notify: tokio::sync::Notify::new(),
        }
    }
}

async fn poll_loop(app: AppHandle, state: Arc<AppState>) {
    loop {
        let cfg = state.config.lock().unwrap().clone();

        if cfg.token.trim().is_empty() {
            *state.error.lock().unwrap() = Some("no-token".to_string());
            tray::clear(&app);
            commands::emit_state(&app, &state);
        } else {
            match aqicn::fetch(&cfg).await {
                Ok(snapshot) => {
                    *state.snapshot.lock().unwrap() = Some(snapshot.clone());
                    *state.error.lock().unwrap() = None;
                    tray::update(&app, &snapshot);
                }
                Err(err) => {
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

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_positioner::init())
        .on_window_event(|window, event| {
            // A menu bar popover closes when it loses focus.
            if let tauri::WindowEvent::Focused(false) = event {
                if window.label() == "main" {
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

            // First run: open the panel so the token step is obvious.
            // AQI_MENUBAR_SHOW is a developer hook for screenshots and e2e checks.
            if cfg.token.trim().is_empty() || std::env::var("AQI_MENUBAR_SHOW").is_ok() {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    tray::show(&handle);
                });
            }

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                poll_loop(handle, state).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::save_config,
            commands::refresh_now,
            commands::search_stations,
        ])
        .run(tauri::generate_context!())
        .expect("error while running AQI Menubar");
}
