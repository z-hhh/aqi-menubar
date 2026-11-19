// Development-only stand-in for the Rust backend, so the panel can be built and
// reviewed in a plain browser (`pnpm dev`). Never reached inside the app.
import type { AppState, Config, StationHit } from "./types";

const params = new URLSearchParams(window.location.search);
const lang = params.get("lang") ?? "auto";
const mode = params.get("mock") ?? "data";
const aqiParam = params.get("aqi");

const ATTRIBUTIONS = [
  {
    name: "Shanghai Environment Monitoring Center(上海市环境监测中心)",
    url: "https://sthj.sh.gov.cn/",
  },
  {
    name: "China National Urban air quality real-time publishing platform (全国城市空气质量实时发布平台)",
    url: "http://106.37.208.233:20035/emcpublish/",
  },
  { name: "U.S. Consulate Shanghai Air Quality Monitor", url: "https://china.usembassy-china.org.cn/" },
  { name: "World Air Quality Index Project", url: "https://waqi.info/" },
];

function baseConfig(): Config {
  return {
    token: mode === "empty" ? "" : "demo",
    location: { kind: "here" },
    refreshMinutes: 30,
    language: lang,
  };
}

export function sampleState(): AppState {
  if (mode === "empty") {
    return { config: baseConfig(), snapshot: null, error: null, resolvedLanguage: "en" };
  }
  const aqi = aqiParam ? Number(aqiParam) : 50;
  return {
    config: baseConfig(),
    resolvedLanguage: "en",
    error: mode === "error" ? "network-error: connection timed out" : null,
    snapshot: {
      aqi,
      station: "Shanghai (上海)",
      stationUrl: "https://aqicn.org/city/shanghai",
      dominant: "pm25",
      updated: new Date(Date.now() - 42 * 60 * 1000).toISOString().replace("Z", "+00:00"),
      stale: mode === "error",
      pollutants: [
        { code: "pm25", value: Math.round(aqi * 0.9) },
        { code: "pm10", value: 20 },
        { code: "o3", value: 23.6 },
        { code: "no2", value: 11.5 },
        { code: "so2", value: 6.1 },
        { code: "co", value: 5.5 },
        { code: "t", value: 11 },
        { code: "h", value: 28 },
        { code: "w", value: 5.6 },
      ],
      attributions: ATTRIBUTIONS,
      fetchedAt: Math.floor(Date.now() / 1000),
    },
  };
}

let current = sampleState();

export async function mockGetState(): Promise<AppState> {
  return current;
}

export async function mockSaveConfig(next: Config): Promise<AppState> {
  current = { ...current, config: next };
  return current;
}

export async function mockSearch(query: string): Promise<StationHit[]> {
  await new Promise((resolve) => setTimeout(resolve, 250));
  const seeds = [
    { uid: 1437, name: "Shanghai (上海)", aqi: 50, country: "CN" },
    { uid: 3505, name: "Jing'an Monitoring Station, Shanghai", aqi: 46, country: "CN" },
    { uid: 8924, name: "Pudong New Area, Shanghai", aqi: 58, country: "CN" },
  ];
  return seeds.map((seed) => ({
    ...seed,
    name: `${seed.name} — ${query}`,
    lat: null,
    lon: null,
  }));
}

export function mockListen(cb: (state: AppState) => void) {
  return Promise.resolve(() => cb(current));
}
