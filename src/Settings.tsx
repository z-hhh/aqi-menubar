import { useEffect, useRef, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useTranslation } from "react-i18next";
import { categoryColor, categoryOf, trayText } from "./aqi";
import { bridge } from "./bridge";
import { TOKEN_URL } from "./Panel";
import type { AppState, Location, StationHit, TrayDisplay } from "./types";

interface Props {
  state: AppState;
  /** Open with the token field focused (the onboarding "I have a token" path). */
  focusToken?: boolean;
  onSaved: (next: AppState) => void;
  onBack: () => void;
}

export default function Settings({ state, focusToken = false, onSaved, onBack }: Props) {
  const { t } = useTranslation();
  const initial = state.config;
  const isWindows = state.platform === "windows";

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
  const [tray, setTray] = useState<TrayDisplay>(initial.tray);
  const [picked, setPicked] = useState<string | null>(
    initial.location.kind === "station" ? t("settings.pickedFallback") : null,
  );
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [checked, setChecked] = useState<"idle" | "busy" | "none">("idle");

  const tokenRef = useRef<HTMLInputElement>(null);
  /** Set once the form is saved, so leaving does not undo the new setting. */
  const committed = useRef(false);

  useEffect(() => {
    if (focusToken) tokenRef.current?.focus();
  }, [focusToken]);

  // Changing the display options shows up in the real menu bar immediately;
  // leaving without saving puts the saved setting back.
  useEffect(() => {
    return () => {
      if (!committed.current) void bridge.previewTray(initial.tray);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  function changeTray(patch: Partial<TrayDisplay>) {
    const next = { ...tray, ...patch };
    setTray(next);
    void bridge.previewTray(next);
  }

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
      tokenRef.current?.focus();
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
        tray,
      });
      committed.current = true;
      onSaved(next);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function checkForUpdates() {
    setChecked("busy");
    const found = await bridge.checkUpdate();
    setChecked(found ? "idle" : "none");
  }

  const category = categoryOf(state.snapshot?.aqi ?? null);
  const preview = trayText(state.snapshot, tray, t);

  return (
    <div className="panel settings">
      <header className="bar">
        <button type="button" className="ghost" onClick={onBack}>
          ‹ {t("settings.back")}
        </button>
        <span className="bar-title">{t("settings.title")}</span>
        <span className="bar-spacer" />
      </header>

      {!token.trim() && (
        <div className="callout">
          <p className="callout-text">{t("settings.tokenGuide")}</p>
          <button type="button" className="primary small" onClick={() => open(TOKEN_URL)}>
            {t("onboard.getToken")} ↗
          </button>
        </div>
      )}

      <div className="field">
        <label className="label" htmlFor="token">
          {t("settings.token")}
        </label>
        <input
          id="token"
          ref={tokenRef}
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
          <button type="button" className="link inline" onClick={() => open(TOKEN_URL)}>
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
                aria-label={t("settings.cityPlaceholder")}
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
            {picked && <span className="hint picked">✓ {picked}</span>}
            {!searching && hits.length === 0 && query.trim() !== "" && (
              <span className="hint">{t("settings.searchHint")}</span>
            )}
          </>
        )}
      </div>

      <div className="field">
        <span className="label">{t("settings.tray")}</span>

        <div className="tray-preview" aria-live="polite" aria-label={t("settings.trayPreview")}>
          {tray.showIcon && (
            <span
              className="tray-dot"
              style={{ background: categoryColor[category] }}
              aria-hidden="true"
            />
          )}
          {preview && <span className="tray-text">{preview}</span>}
        </div>

        <select
          aria-label={t("settings.trayMetric")}
          value={tray.metric}
          onChange={(e) => changeTray({ metric: e.target.value as TrayDisplay["metric"] })}
        >
          <option value="aqi">{t("settings.trayAqi")}</option>
          <option value="pm25">{t("settings.trayPm25")}</option>
          {!isWindows && <option value="none">{t("settings.trayIconOnly")}</option>}
        </select>

        <label className="check">
          <input
            type="checkbox"
            checked={tray.showLabel}
            onChange={(e) => changeTray({ showLabel: e.target.checked })}
          />
          <span>{t("settings.trayLabel")}</span>
        </label>
        <label className="check">
          <input
            type="checkbox"
            checked={tray.showCategory}
            onChange={(e) => changeTray({ showCategory: e.target.checked })}
          />
          <span>{t("settings.trayCategory")}</span>
        </label>
        <label className="check">
          <input
            type="checkbox"
            checked={tray.showStation}
            onChange={(e) => changeTray({ showStation: e.target.checked })}
          />
          <span>{t("settings.trayStation")}</span>
        </label>
        {!isWindows && (
          <label className="check">
            <input
              type="checkbox"
              checked={tray.showIcon}
              onChange={(e) => changeTray({ showIcon: e.target.checked })}
            />
            <span>{t("settings.trayDot")}</span>
          </label>
        )}
        <span className="hint">{isWindows ? t("settings.trayWindowsHint") : t("settings.trayHint")}</span>
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

      <div className="field">
        <span className="label">{t("settings.updates")}</span>
        <div className="row">
          <button
            type="button"
            className="ghost"
            disabled={checked === "busy"}
            onClick={() => void checkForUpdates()}
          >
            {checked === "busy" ? t("update.checking") : t("update.check")}
          </button>
        </div>
        <span className="hint">
          {state.update
            ? t("update.available", { version: state.update.version })
            : checked === "none"
              ? t("update.upToDate")
              : t("update.autoHint")}
        </span>
      </div>

      {error && <p className="error">{error}</p>}

      <button type="button" className="primary wide" disabled={busy} onClick={() => void save()}>
        {busy ? t("settings.saving") : t("settings.save")}
      </button>
    </div>
  );
}

function open(url: string) {
  void openUrl(url).catch(() => undefined);
}
