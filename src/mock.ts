// Development-only stand-in for the Rust backend, so the panel can be built and
// reviewed in a plain browser (`pnpm dev`). Never reached inside the app.
import type { AppState, Config, ForecastDay, Obs, SaveConfig, StationHit } from "./types";

const params = new URLSearchParams(window.location.search);
const lang = params.get("lang") ?? "auto";
const mode = params.get("mock") ?? "data";
const aqiParam = params.get("aqi");
const updateParam = params.get("update");

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
    location: { kind: "here" },
    refreshMinutes: 30,
    language: lang,
    tray: {
      metric: (params.get("metric") as "aqi" | "pm25" | "none") ?? "aqi",
      showLabel: params.get("label") === "1",
      showCategory: params.get("level") !== "0",
      showStation: params.get("station") === "1",
      showIcon: params.get("dot") !== "0",
      colorText: params.get("color") === "1",
    },
  };
}

const STATION = "Shanghai (上海)";

/**
 * A station that publishes every three hours, as many aqicn stations do: the
 * API repeats the last measurement until a new one lands, so the cache gets a
 * point every 3 h however often we poll. Four readings, no line to draw unless
 * the chart tolerates this rhythm.
 */
function sparseHistory(aqi: number): Obs[] {
  const now = Date.now();
  const shift = [40, 28, 12, 0];
  return shift.map((offset, i) => ({
    station: STATION,
    observedAt: Math.floor((now - (shift.length - 1 - i) * 3 * 3_600_000) / 1000),
    aqi: Math.max(3, aqi + offset),
    pm25: null,
  }));
}

/**
 * A plausible day curve so the panel can be reviewed in a browser. Real
 * readings only accumulate while the app runs; this is never reached inside
 * the app.
 */
function mockHistory(aqi: number): Obs[] {
  const now = Date.now();
  const step = 30 * 60_000;
  const start = now - 40 * 60 * 60_000;
  const out: Obs[] = [];
  for (let t = start; t <= now; t += step) {
    const hour = new Date(t).getHours();
    // Was the app closed overnight? Leave a real gap, not a straight line.
    if (hour >= 1 && hour < 6 && t < now - 12 * 3_600_000) continue;
    const swing = Math.sin(((hour - 4) / 24) * Math.PI * 2) * 22;
    const drift = Math.sin((t - start) / (12 * 3_600_000)) * 11;
    out.push({
      station: STATION,
      observedAt: Math.floor(t / 1000),
      aqi: Math.max(3, Math.round(aqi + swing + drift)),
      pm25: null,
    });
  }
  // End on the reading the panel shows, so the dot sits on the line.
  const last = out[out.length - 1];
  if (last) {
    const shift = aqi - (last.aqi ?? aqi);
    for (const point of out) point.aqi = Math.max(3, (point.aqi ?? aqi) + shift);
  }
  return out;
}

/** aqicn's real shape: today plus a few days, min/avg/max per day. */
function mockForecast(aqi: number): ForecastDay[] {
  const out: ForecastDay[] = [];
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  const shape = [1.05, 0.82, 1.4, 1.15, 0.7];
  for (let i = 0; i < shape.length; i++) {
    const day = new Date(today.getTime() + i * 86_400_000);
    const avg = Math.round(aqi * shape[i]);
    const month = String(day.getMonth() + 1).padStart(2, "0");
    const date = String(day.getDate()).padStart(2, "0");
    out.push({
      day: `${day.getFullYear()}-${month}-${date}`,
      pm25: { avg, min: Math.round(avg * 0.62), max: Math.round(avg * 1.36) },
      pm10: { avg: Math.round(avg * 0.5), min: Math.round(avg * 0.3), max: Math.round(avg * 0.8) },
    });
  }
  return out;
}

function updateInfo() {
  return updateParam
    ? { version: updateParam, currentVersion: "0.1.0", notes: "Fixes and a smaller tray title." }
    : null;
}

export function sampleState(): AppState {
  if (mode === "empty") {
    return {
      config: baseConfig(),
      tokenSet: false,
      snapshot: null,
      error: null,
      resolvedLanguage: "en",
      platform: "macos",
      update: null,
      installing: false,
      history: [],
    };
  }
  const aqi = aqiParam ? Number(aqiParam) : 50;
  return {
    config: baseConfig(),
    tokenSet: true,
    resolvedLanguage: "en",
    error: mode === "error" ? "network-error: connection timed out" : null,
    platform: "macos",
    update: updateInfo(),
    installing: false,
    history: mode === "sparse" ? sparseHistory(aqi) : mockHistory(aqi),
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
      observedAt: Math.floor(Date.now() / 1000) - 62 * 60,
      forecast: mockForecast(aqi),
      fetchedAt: Math.floor(Date.now() / 1000),
    },
  };
}

let current = sampleState();

export async function mockGetState(): Promise<AppState> {
  return current;
}

export async function mockSaveConfig(next: SaveConfig): Promise<AppState> {
  const { token, ...config } = next;
  current = {
    ...current,
    config,
    tokenSet: token?.trim() ? true : current.tokenSet,
  };
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
