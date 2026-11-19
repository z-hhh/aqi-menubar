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
  showStation: boolean;
  showIcon: boolean;
}

export interface Config {
  token: string;
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

export interface AppState {
  config: Config;
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
