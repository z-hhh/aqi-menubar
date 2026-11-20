import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AppState, SaveConfig, StationHit, TrayDisplay, UpdateInfo } from "./types";
import { mockGetState, mockListen, mockSaveConfig, mockSearch } from "./mock";

const hasTauri = "__TAURI_INTERNALS__" in window;

/** True only under `pnpm dev` in a normal browser, where mock.ts answers. */
export const isMock = import.meta.env.DEV && !hasTauri;

export const bridge = {
  getState: (): Promise<AppState> =>
    hasTauri ? invoke<AppState>("get_state") : mockGetState(),

  saveConfig: (next: SaveConfig): Promise<AppState> =>
    hasTauri ? invoke<AppState>("save_config", { next }) : mockSaveConfig(next),

  refreshNow: (): Promise<void> =>
    hasTauri ? invoke<void>("refresh_now") : Promise.resolve(),

  searchStations: (query: string): Promise<StationHit[]> =>
    hasTauri ? invoke<StationHit[]>("search_stations", { query }) : mockSearch(query),

  /** Show a display setting in the real menu bar without saving it. */
  previewTray: (display: TrayDisplay): Promise<void> =>
    hasTauri ? invoke<void>("preview_tray", { display }) : Promise.resolve(),

  checkUpdate: (): Promise<UpdateInfo | null> =>
    hasTauri ? invoke<UpdateInfo | null>("check_update") : Promise.resolve(null),

  installUpdate: (): Promise<void> =>
    hasTauri ? invoke<void>("install_update") : Promise.reject("not available"),

  onState: (cb: (state: AppState) => void): Promise<UnlistenFn> =>
    hasTauri ? listen<AppState>("aqi://state", (event) => cb(event.payload)) : mockListen(cb),

  onUpdate: (cb: (update: UpdateInfo | null) => void): Promise<UnlistenFn> =>
    hasTauri
      ? listen<UpdateInfo | null>("aqi://update", (event) => cb(event.payload))
      : Promise.resolve(() => undefined as void),

  onOpenSettings: (cb: () => void): Promise<UnlistenFn> =>
    hasTauri ? listen("aqi://open-settings", () => cb()) : Promise.resolve(() => undefined as void),

  onCheckUpdates: (cb: () => void): Promise<UnlistenFn> =>
    hasTauri
      ? listen("aqi://check-updates", () => cb())
      : Promise.resolve(() => undefined as void),
};
