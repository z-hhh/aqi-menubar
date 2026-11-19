export type Location =
  | { kind: "here" }
  | { kind: "city"; query: string }
  | { kind: "station"; uid: number }
  | { kind: "geo"; lat: number; lon: number };

export interface Config {
  token: string;
  location: Location;
  refreshMinutes: number;
  language: string;
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

export interface AppState {
  config: Config;
  snapshot: Snapshot | null;
  error: string | null;
  resolvedLanguage: string;
}

export interface StationHit {
  uid: number;
  name: string;
  aqi: number | null;
  country: string | null;
  lat: number | null;
  lon: number | null;
}
