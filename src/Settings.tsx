import { useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useTranslation } from "react-i18next";
import { bridge } from "./bridge";
import type { AppState, Config, Location, StationHit } from "./types";

interface Props {
  initial: Config;
  onSaved: (next: AppState) => void;
  onBack: () => void;
}

const TOKEN_URL = "https://aqicn.org/data-platform/token/";

export default function Settings({ initial, onSaved, onBack }: Props) {
  const { t } = useTranslation();
  const initialMode = initial.location.kind === "city" || initial.location.kind === "station"
    ? "city"
    : "here";
  const initialQuery =
    initial.location.kind === "city"
      ? initial.location.query
      : initial.location.kind === "station"
        ? `@${initial.location.uid}`
        : "";

  const [token, setToken] = useState(initial.token);
  const [mode, setMode] = useState<"here" | "city">(initialMode);
  const [query, setQuery] = useState(initialQuery);
  const [hits, setHits] = useState<StationHit[]>([]);
  const [searching, setSearching] = useState(false);
  const [refresh, setRefresh] = useState(initial.refreshMinutes);
  const [language, setLanguage] = useState(initial.language);
  const [picked, setPicked] = useState<string | null>(
    initial.location.kind === "station" ? t("settings.pickedFallback") : null,
  );
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function search() {
    const term = query.trim();
    if (!term) return;
    setSearching(true);
    setError(null);
    try {
      setHits(await bridge.searchStations(term));
    } catch (err) {
      setError(String(err));
    } finally {
      setSearching(false);
    }
  }

  async function save() {
    if (!token.trim()) {
      setError(t("settings.tokenNeeded"));
      return;
    }
    const location: Location =
      mode === "here" ? { kind: "here" } : { kind: "city", query: query.trim() };
    if (mode === "city" && !query.trim()) {
      setError(t("settings.locationNeeded"));
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const next = await bridge.saveConfig({
        token: token.trim(),
        location,
        refreshMinutes: refresh,
        language,
      });
      onSaved(next);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="panel settings">
      <header className="bar">
        <button type="button" className="ghost" onClick={onBack}>
          ‹ {t("settings.back")}
        </button>
        <span className="bar-title">{t("settings.title")}</span>
        <span className="bar-spacer" />
      </header>

      <div className="field">
        <label className="label" htmlFor="token">
          {t("settings.token")}
        </label>
        <input
          id="token"
          type="text"
          aria-describedby="token-hint"
          value={token}
          spellCheck={false}
          autoCapitalize="off"
          autoCorrect="off"
          placeholder={t("settings.tokenPlaceholder")}
          onChange={(e) => setToken(e.target.value)}
        />
        <span className="hint" id="token-hint">
          {t("settings.tokenHint")}{" "}
          <button type="button" className="link inline" onClick={() => openUrl(TOKEN_URL)}>
            aqicn.org
          </button>
        </span>
      </div>

      <div className="field">
        <span className="label">{t("settings.location")}</span>
        <div className="segmented">
          <button
            type="button"
            className={mode === "here" ? "on" : ""}
            onClick={() => setMode("here")}
          >
            {t("settings.locHere")}
          </button>
          <button
            type="button"
            className={mode === "city" ? "on" : ""}
            onClick={() => setMode("city")}
          >
            {t("settings.locCity")}
          </button>
        </div>
        {mode === "city" && (
          <>
            <div className="row">
              <input
                type="text"
                value={query}
                placeholder={t("settings.cityPlaceholder")}
                onChange={(e) => {
                  setQuery(e.target.value);
                  setPicked(null);
                }}
                onKeyDown={(e) => {
                  if (e.key === "Enter") void search();
                }}
              />
              <button type="button" className="ghost" onClick={() => void search()}>
                {searching ? t("settings.searching") : t("settings.search")}
              </button>
            </div>
            {hits.length > 0 && (
              <ul className="hits">
                {hits.slice(0, 6).map((hit) => (
                  <li key={hit.uid}>
                    <button
                      type="button"
                      className="hit"
                      onClick={() => {
                        setQuery(`@${hit.uid}`);
                        setPicked(hit.name);
                        setHits([]);
                      }}
                    >
                      <span className="hit-name">{hit.name}</span>
                      <span className="hit-aqi">{hit.aqi ?? "–"}</span>
                    </button>
                  </li>
                ))}
              </ul>
            )}
            {picked && (
              <span className="hint picked">✓ {picked}</span>
            )}
            {!searching && hits.length === 0 && query.trim() !== "" && (
              <span className="hint">{t("settings.searchHint")}</span>
            )}
          </>
        )}
      </div>

      <div className="field">
        <label className="label" htmlFor="refresh">
          {t("settings.refresh")}
        </label>
        <select
          id="refresh"
          value={refresh}
          onChange={(e) => setRefresh(Number(e.target.value))}
        >
          {[5, 10, 15, 30, 60].map((minutes) => (
            <option key={minutes} value={minutes}>
              {t("settings.minutes", { n: minutes })}
            </option>
          ))}
        </select>
      </div>

      <div className="field">
        <label className="label" htmlFor="language">
          {t("settings.language")}
        </label>
        <select id="language" value={language} onChange={(e) => setLanguage(e.target.value)}>
          <option value="auto">{t("settings.langAuto")}</option>
          <option value="en">English</option>
          <option value="zh-CN">简体中文</option>
          <option value="ja">日本語</option>
        </select>
      </div>

      {error && <p className="error">{error}</p>}

      <button type="button" className="primary wide" disabled={busy} onClick={() => void save()}>
        {busy ? t("settings.saving") : t("settings.save")}
      </button>
    </div>
  );
}
