use crate::{
    aqicn::{self, Snapshot, StationHit},
    config::Config,
    tray, AppState,
};
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

pub const EVENT_STATE: &str = "aqi://state";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStateDto {
    pub config: Config,
    pub snapshot: Option<Snapshot>,
    pub error: Option<String>,
    /// The language actually in effect ("auto" already resolved).
    pub resolved_language: String,
}

pub fn dto(state: &AppState) -> AppStateDto {
    let config = state.config.lock().unwrap().clone();
    AppStateDto {
        resolved_language: config.resolved_language(),
        config,
        snapshot: state.snapshot.lock().unwrap().clone(),
        error: state.error.lock().unwrap().clone(),
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
    next: Config,
) -> Result<AppStateDto, String> {
    next.save(&app).map_err(|e| e.to_string())?;
    *state.config.lock().unwrap() = next.clone();
    tray::refresh_menu(&app, &next);
    state.notify.notify_one();
    Ok(dto(state.inner()))
}

#[tauri::command]
pub fn refresh_now(state: State<'_, Arc<AppState>>) {
    state.notify.notify_one();
}

#[tauri::command]
pub async fn search_stations(
    state: State<'_, Arc<AppState>>,
    query: String,
) -> Result<Vec<StationHit>, String> {
    let cfg = state.config.lock().unwrap().clone();
    aqicn::search(&cfg, &query).await.map_err(|e| e.to_string())
}
