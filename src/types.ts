export type Location =
  | { kind: "here" }
  | { kind: "city"; query: string }
  | { kind: "station"; uid: number }
  | { kind: "geo"; lat: number; lon: number };

export type TrayMetric = "aqi" | "pm25" | "none";

/** Mirrors `config::TrayDisplay` on the Rust side. */
export interface TrayDisplay {
  metric: TrayMetric;
  showLabel: boolean;
  /** Append the quality level, e.g. "51 · 良". */
  showCategory: boolean;
  showStation: boolean;
  showIcon: boolean;
  /** Draw the menu bar text in the level colour (macOS). */
  colorText: boolean;
}

/**
 * The saved settings as the UI sees them. The aqicn token is deliberately
 * absent: it stays in the Rust side and is never sent to the web view.
 */
export interface Config {
  location: Location;
  refreshMinutes: number;
  language: string;
  tray: TrayDisplay;
}

export interface Pollutant {
  code: string;
  value: number;
}

export interface Attribution {
  name: string;
  url: string | null;
}

export interface Snapshot {
  aqi: number | null;
  station: string;
  stationUrl: string | null;
  dominant: string | null;
  updated: string | null;
  stale: boolean;
  pollutants: Pollutant[];
  attributions: Attribution[];
  fetchedAt: number;
}

export interface UpdateInfo {
  version: string;
  currentVersion: string;
  notes: string | null;
}

/** What the settings screen sends back. `token` is present only when the
 *  person typed a new one; without it the backend keeps the saved token. */
export interface SaveConfig extends Config {
  token?: string;
}

export interface AppState {
  config: Config;
  /** A token is stored. The token itself is never exposed. */
  tokenSet: boolean;
  snapshot: Snapshot | null;
  error: string | null;
  resolvedLanguage: string;
  /** "macos" | "windows" | "linux" */
  platform: string;
  update: UpdateInfo | null;
  installing: boolean;
}

export interface StationHit {
  uid: number;
  name: string;
  aqi: number | null;
  country: string | null;
  lat: number | null;
  lon: number | null;
}
