import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AppState, Config, StationHit } from "./types";
import { mockGetState, mockListen, mockSaveConfig, mockSearch } from "./mock";

const hasTauri = "__TAURI_INTERNALS__" in window;

/** True only under `pnpm dev` in a normal browser, where mock.ts answers. */
export const isMock = import.meta.env.DEV && !hasTauri;

export const bridge = {
  getState: (): Promise<AppState> =>
    hasTauri ? invoke<AppState>("get_state") : mockGetState(),

  saveConfig: (next: Config): Promise<AppState> =>
    hasTauri ? invoke<AppState>("save_config", { next }) : mockSaveConfig(next),

  refreshNow: (): Promise<void> =>
    hasTauri ? invoke<void>("refresh_now") : Promise.resolve(),

  searchStations: (query: string): Promise<StationHit[]> =>
    hasTauri ? invoke<StationHit[]>("search_stations", { query }) : mockSearch(query),

  onState: (cb: (state: AppState) => void): Promise<UnlistenFn> =>
    hasTauri ? listen<AppState>("aqi://state", (event) => cb(event.payload)) : mockListen(cb),

  onOpenSettings: (cb: () => void): Promise<UnlistenFn> =>
    hasTauri ? listen("aqi://open-settings", () => cb()) : Promise.resolve(() => undefined as void),
};
