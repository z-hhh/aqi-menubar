use crate::{
    aqicn::{self, Snapshot, StationHit},
    config::{Config, TrayDisplay},
    tray, AppState, EVENT_UPDATE,
};
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_updater::UpdaterExt;

pub const EVENT_STATE: &str = "aqi://state";

/// A release newer than this build, as reported by the updater feed.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub current_version: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStateDto {
    /// The saved settings, with the token blanked: the token never leaves Rust.
    pub config: Config,
    /// Whether a token is stored, so the UI can tell "none yet" from "saved".
    pub token_set: bool,
    pub snapshot: Option<Snapshot>,
    pub error: Option<String>,
    /// The language actually in effect ("auto" already resolved).
    pub resolved_language: String,
    /// "macos" | "windows" | "linux"
    pub platform: String,
    pub update: Option<UpdateInfo>,
    pub installing: bool,
}

pub fn dto(state: &AppState) -> AppStateDto {
    let stored = state.config.lock().unwrap().clone();
    AppStateDto {
        resolved_language: stored.resolved_language(),
        token_set: stored.has_token(),
        config: stored.without_token(),
        snapshot: state.snapshot.lock().unwrap().clone(),
        error: state.error.lock().unwrap().clone(),
        platform: std::env::consts::OS.to_string(),
        update: state.update.lock().unwrap().clone(),
        installing: *state.installing.lock().unwrap(),
    }
}

pub fn emit_state(app: &AppHandle, state: &AppState) {
    let _ = app.emit(EVENT_STATE, dto(state));
}

#[tauri::command]
pub fn get_state(state: State<'_, Arc<AppState>>) -> AppStateDto {
    dto(state.inner())
}

#[tauri::command]
pub fn save_config(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    mut next: Config,
) -> Result<AppStateDto, String> {
    // The settings screen never sees the saved token, so it does not send one
    // back: keep what is already stored instead of blanking it.
    next.keep_token_from(&state.config.lock().unwrap());
    next.save(&app).map_err(|e| e.to_string())?;
    *state.config.lock().unwrap() = next.clone();
    tray::refresh_menu(&app, &next);
    // Apply the tray display change right away instead of waiting for the
    // next poll, so the menu bar reacts the moment the user saves.
    let lang = next.resolved_language();
    match state.snapshot.lock().unwrap().as_ref() {
        Some(snapshot) => tray::update(&app, snapshot, &next.tray, &lang),
        None => tray::clear(&app, &next.tray, &lang),
    }
    state.notify.notify_one();
    Ok(dto(state.inner()))
}

#[tauri::command]
pub fn refresh_now(state: State<'_, Arc<AppState>>) {
    state.notify.notify_one();
}

/// Show a display configuration in the real menu bar without persisting it,
/// so the settings screen can preview as the user toggles options.
#[tauri::command]
pub fn preview_tray(app: AppHandle, state: State<'_, Arc<AppState>>, display: TrayDisplay) {
    let lang = state.config.lock().unwrap().resolved_language();
    match state.snapshot.lock().unwrap().as_ref() {
        Some(snapshot) => tray::update(&app, snapshot, &display, &lang),
        None => tray::clear(&app, &display, &lang),
    }
}

#[tauri::command]
pub async fn search_stations(
    state: State<'_, Arc<AppState>>,
    query: String,
) -> Result<Vec<StationHit>, String> {
    let cfg = state.config.lock().unwrap().clone();
    aqicn::search(&cfg, &query).await.map_err(|e| e.to_string())
}

/// Ask the release feed now (the tray menu and the panel's button both use it).
#[tauri::command]
pub async fn check_update(app: AppHandle) -> Option<UpdateInfo> {
    crate::check_for_update(&app, true).await
}

/// Download, verify and install the release the updater found, then relaunch.
#[tauri::command]
pub async fn install_update(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    {
        let mut installing = state.installing.lock().unwrap();
        if *installing {
            return Err("already-installing".into());
        }
        *installing = true;
    }
    emit_state(&app, state.inner());

    let result = async {
        let updater = app.updater().map_err(|e| e.to_string())?;
        let update = updater
            .check()
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "no-update".to_string())?;
        update
            .download_and_install(|_chunk, _total| {}, || {})
            .await
            .map_err(|e| e.to_string())?;
        Ok::<(), String>(())
    }
    .await;

    if let Err(err) = &result {
        *state.installing.lock().unwrap() = false;
        *state.update.lock().unwrap() = None;
        let _ = app.emit(EVENT_UPDATE, Option::<UpdateInfo>::None);
        emit_state(&app, state.inner());
        return Err(err.clone());
    }

    app.restart();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::Config, AppState};

    #[test]
    fn the_state_handed_to_the_ui_carries_no_token() {
        let cfg = Config {
            token: "secret-token".into(),
            ..Config::default()
        };
        let state = AppState::new(cfg);
        let dto = dto(&state);
        assert!(dto.token_set);
        assert!(dto.config.token.is_empty());

        // What actually crosses the IPC bridge.
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["tokenSet"], true);
        assert_eq!(json["config"]["token"], "");
        assert!(!json.to_string().contains("secret-token"));
    }

    #[test]
    fn without_a_token_the_ui_is_told_there_is_none() {
        let state = AppState::new(Config::default());
        let dto = dto(&state);
        assert!(!dto.token_set);
        assert!(dto.config.token.is_empty());
    }
}
