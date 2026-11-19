import type { Snapshot } from "./types";

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
