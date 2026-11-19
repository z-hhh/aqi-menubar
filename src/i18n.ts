import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import en from "./locales/en.json";
import zhCN from "./locales/zh-CN.json";
import ja from "./locales/ja.json";

export const SUPPORTED = ["en", "zh-CN", "ja"] as const;

export function resolveLanguage(setting: string): string {
  if (setting !== "auto") {
    return (SUPPORTED as readonly string[]).includes(setting) ? setting : "en";
  }
  const raw = (navigator.language || "en").toLowerCase();
  if (raw.startsWith("zh")) return "zh-CN";
  if (raw.startsWith("ja")) return "ja";
  return "en";
}

i18n.use(initReactI18next).init({
  resources: {
    en: { translation: en },
    "zh-CN": { translation: zhCN },
    ja: { translation: ja },
  },
  lng: "en",
  fallbackLng: "en",
  interpolation: { escapeValue: false },
});

export default i18n;
