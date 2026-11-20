import type { TFunction } from "i18next";
import type { Snapshot, TrayDisplay } from "./types";

export type Category =
  | "good"
  | "moderate"
  | "sensitive"
  | "unhealthy"
  | "veryUnhealthy"
  | "hazardous"
  | "unknown";

/** US EPA bands, the ones aqicn leads with. */
export function categoryOf(aqi: number | null): Category {
  if (aqi === null || aqi < 0) return "unknown";
  if (aqi <= 50) return "good";
  if (aqi <= 100) return "moderate";
  if (aqi <= 150) return "sensitive";
  if (aqi <= 200) return "unhealthy";
  if (aqi <= 300) return "veryUnhealthy";
  return "hazardous";
}

export const categoryColor: Record<Category, string> = {
  good: "#00c853",
  moderate: "#ffd60a",
  sensitive: "#ff7e00",
  unhealthy: "#ff3b30",
  veryUnhealthy: "#8f3f97",
  hazardous: "#7e0023",
  unknown: "#9a9a9a",
};

/** Yellow and orange need dark text; the rest read best on white. */
export const categoryInk: Record<Category, string> = {
  good: "#06310f",
  moderate: "#3a2c00",
  sensitive: "#2a1300",
  unhealthy: "#ffffff",
  veryUnhealthy: "#ffffff",
  hazardous: "#ffffff",
  unknown: "#ffffff",
};

/**
 * Text-safe tones for the level words. The EPA fills are made for large
 * areas: yellow on white and dark green on a dark panel both vanish. Mirrors
 * `icons::text_color` on the Rust side so the preview matches the menu bar.
 */
export const categoryText: Record<Category, { light: string; dark: string }> = {
  good: { light: "#10742f", dark: "#4cd96c" },
  moderate: { light: "#8a5f00", dark: "#ffd60a" },
  sensitive: { light: "#c45000", dark: "#ff9f2e" },
  unhealthy: { light: "#c61b14", dark: "#ff695e" },
  veryUnhealthy: { light: "#7a2e86", dark: "#c486cc" },
  hazardous: { light: "#7e0023", dark: "#ff7a95" },
  unknown: { light: "inherit", dark: "inherit" },
};

const CJK = /[぀-ヿ㐀-䶿一-鿿豈-﫿]/;

/**
 * aqicn names stations "English (本地名)". Chinese and Japanese readers go by
 * the local name; English keeps the whole feed string. Mirrors
 * `tray::station_display`.
 */
export function stationDisplay(name: string | null | undefined, language: string): string {
  const trimmed = (name ?? "").trim();
  if (!trimmed) return "";
  const match = /^(.*?)[(（]([^()（）]+)[)）]$/.exec(trimmed);
  const local = match?.[2]?.trim() ?? "";
  if (!match || !local || !match[1].trim()) return trimmed;
  if ((language === "zh-CN" || language === "ja") && CJK.test(local)) return local;
  return trimmed;
}

const UNITS: Record<string, string> = {
  pm25: "µg/m³",
  pm10: "µg/m³",
  o3: "ppb",
  no2: "ppb",
  so2: "ppb",
  co: "ppm",
  t: "°C",
  h: "%",
  p: "hPa",
  w: "m/s",
};

export function unitOf(code: string): string {
  return UNITS[code] ?? "";
}

export function formatTemp(value: number, unit: "c" | "f"): string {
  return unit === "f" ? `${Math.round((value * 9) / 5 + 32)}°F` : `${Math.round(value)}°C`;
}

/** aqicn sends "2026-11-19T10:00:00+08:00"; show the station's local time. */
export function readingTime(snapshot: Snapshot | null, locale: string): string {
  if (!snapshot?.updated) return "";
  const date = new Date(snapshot.updated);
  if (Number.isNaN(date.getTime())) return snapshot.updated;
  return new Intl.DateTimeFormat(locale, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

function pollutantValue(snapshot: Snapshot | null, code: string): number | null {
  const hit = snapshot?.pollutants.find((p) => p.code === code);
  return hit && Number.isFinite(hit.value) && hit.value >= 0 ? hit.value : null;
}

function truncate(value: string, max: number): string {
  const chars = Array.from(value);
  return chars.length <= max ? value : `${chars.slice(0, max - 1).join("").trimEnd()}…`;
}

function metricText(
  snapshot: Snapshot | null,
  metric: TrayDisplay["metric"],
): { value: string; label: string } | null {
  const aqi =
    snapshot?.aqi !== null && snapshot?.aqi !== undefined && snapshot.aqi >= 0
      ? snapshot.aqi
      : null;
  if (metric === "pm25") {
    const pm25 = pollutantValue(snapshot, "pm25");
    if (pm25 !== null) return { value: String(Math.round(pm25)), label: "PM2.5" };
  }
  if (metric === "none") return null;
  return aqi === null ? null : { value: String(aqi), label: "AQI" };
}

function aqiValue(snapshot: Snapshot | null): number | null {
  return snapshot?.aqi !== null && snapshot?.aqi !== undefined && snapshot.aqi >= 0
    ? snapshot.aqi
    : null;
}

/**
 * The menu bar title a display setting produces, given a reading. Mirrors
 * `tray::texts` in Rust — the two must agree; used for the live preview.
 * The level word comes from the locale files (`aqiShort`), the same words the
 * Rust side uses for the real menu bar.
 */
export function trayText(
  snapshot: Snapshot | null,
  display: TrayDisplay,
  t: TFunction,
  language: string,
): string {
  const station = truncate(stationDisplay(snapshot?.station, language), 18);
  const hit = metricText(snapshot, display.metric);
  const aqi = aqiValue(snapshot);
  const category = categoryOf(aqi);
  const level =
    display.showCategory && category !== "unknown" ? t(`aqiShort.${category}`) : null;

  if (!hit) {
    // Icon-only: nothing beside the dot while there is a reading; a dash is
    // clearer than a bare grey dot when there is none.
    if (aqi === null) return "–";
    if (display.metric === "none" && display.showIcon) return "";
    return level ? `${aqi} · ${level}` : String(aqi);
  }

  const parts = [display.showLabel ? `${hit.label} ${hit.value}` : hit.value];
  if (level) parts.push(level);
  if (display.showStation && station) parts.push(station);
  return parts.join(" · ");
}
