import { useEffect, useRef, useState, type CSSProperties } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useTranslation } from "react-i18next";
import { categoryColor, categoryOf, categoryText, trayText } from "./aqi";
import { bridge } from "./bridge";
import { TOKEN_URL } from "./Panel";
import type { AppState, Location, SaveConfig, StationHit, TrayDisplay } from "./types";

interface Props {
  state: AppState;
  /** Open with the token field focused (the onboarding "I have a token" path). */
  focusToken?: boolean;
  /** Called after each auto-save with the state the backend returned. */
  onSaved: (next: AppState) => void;
  onBack: () => void;
}

/** Quiet period after the last edit before it is written. Short enough to feel
 *  immediate, long enough that a typed city name is not saved letter by letter. */
const AUTOSAVE_DELAY_MS = 400;
/** How long "Saved" stays in the header. */
const SAVED_FLASH_MS = 1600;

function locationOf(mode: "here" | "city", query: string): Location {
  return mode === "here" ? { kind: "here" } : { kind: "city", query: query.trim() };
}

/** Everything that can make the screen differ from what is stored. The token is
 *  left out: a typed token is always written, the rest only when it changed. */
function signature(payload: SaveConfig): string {
  return JSON.stringify({
    location: payload.location,
    refreshMinutes: payload.refreshMinutes,
    language: payload.language,
    tray: payload.tray,
  });
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

  const [token, setToken] = useState("");
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
  const [tokenSet, setTokenSet] = useState(state.tokenSet);
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState<"idle" | "saving" | "saved" | "error">("idle");
  const [checked, setChecked] = useState<"idle" | "busy" | "none">("idle");

  const tokenRef = useRef<HTMLInputElement>(null);
  /** The token field has focus, so a freshly saved token may stay in it. */
  const tokenFocused = useRef(false);
  /** The token value the last successful save wrote, so leaving the field can
   *  clear it without asking the backend. */
  const writtenToken = useRef("");
  /** The edit waiting for the debounce to run out — also flushed on leaving. */
  const pending = useRef<SaveConfig | null>(null);
  /** The edit that failed, so the retry link has something to send again. */
  const failed = useRef<SaveConfig | null>(null);
  const alive = useRef(true);
  const writeRef = useRef<(payload: SaveConfig) => void>(() => undefined);
  /** What the settings screen opened with. Opening is not an edit: a screen
   *  that still shows these values has nothing to write. */
  const known = useRef(
    signature({
      location: locationOf(initialMode, initialQuery),
      refreshMinutes: initial.refreshMinutes,
      language: initial.language,
      tray: initial.tray,
    }),
  );

  useEffect(() => {
    if (focusToken) tokenRef.current?.focus();
  }, [focusToken]);

  useEffect(() => {
    if (status !== "saved") return;
    const id = window.setTimeout(() => {
      if (alive.current) setStatus("idle");
    }, SAVED_FLASH_MS);
    return () => window.clearTimeout(id);
  }, [status]);

  async function write(payload: SaveConfig) {
    setStatus("saving");
    const typed = payload.token?.trim();
    try {
      const next = await bridge.saveConfig(payload);
      known.current = signature(payload);
      if (typed) writtenToken.current = typed;
      failed.current = null;
      if (!alive.current) return;
      setTokenSet(next.tokenSet);
      setError(null);
      setStatus("saved");
      onSaved(next);
      // A saved token does not stay on screen once the field is left alone.
      if (typed && !tokenFocused.current) setToken("");
    } catch (err) {
      failed.current = payload;
      if (!alive.current) return;
      setError(String(err));
      setStatus("error");
    }
  }
  writeRef.current = (payload) => void write(payload);

  useEffect(() => {
    // Nothing to keep for a city that was emptied: the stored location stays.
    if (mode === "city" && !query.trim()) return;

    const typed = token.trim();
    const payload: SaveConfig = {
      ...(typed ? { token: typed } : {}),
      location: locationOf(mode, query),
      refreshMinutes: refresh,
      language,
      tray,
    };
    // Same as what is already stored (or untouched since opening): no write.
    if (!typed && signature(payload) === known.current) return;

    pending.current = payload;
    const id = window.setTimeout(() => {
      pending.current = null;
      writeRef.current(payload);
    }, AUTOSAVE_DELAY_MS);
    return () => window.clearTimeout(id);
    // writeRef carries the latest render's write without re-scheduling on it.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [token, mode, query, refresh, language, tray]);

  // Leaving with an edit still inside its quiet period writes it before the
  // screen goes away, so "change and close" loses nothing.
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
      const tail = pending.current;
      if (tail) void bridge.saveConfig(tail).catch(() => undefined);
    };
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
    try {
      setHits(await bridge.searchStations(term));
    } catch (err) {
      setError(String(err));
    } finally {
      setSearching(false);
    }
  }

  async function checkForUpdates() {
    setChecked("busy");
    const found = await bridge.checkUpdate();
    setChecked(found ? "idle" : "none");
  }

  const category = categoryOf(state.snapshot?.aqi ?? null);
  const preview = trayText(state.snapshot, tray, t, state.resolvedLanguage);
  const previewColor = tray.colorText ? categoryText[category] : null;

  return (
    <div className="panel settings">
      <header className="bar">
        <button type="button" className="ghost" onClick={onBack}>
          ‹ {t("settings.back")}
        </button>
        <span className="bar-title">{t("settings.title")}</span>
        <span className="bar-spacer" />
        <span className={`save-state ${status}`} aria-live="polite">
          {status === "saving"
            ? t("settings.saving")
            : status === "saved"
              ? t("settings.saved")
              : null}
        </span>
      </header>

      {error && (
        <p className="save-error" role="alert">
          <span>{t("settings.saveFailed", { error })}</span>
          <button
            type="button"
            className="link inline"
            onClick={() => {
              const again = failed.current;
              if (again) writeRef.current(again);
            }}
          >
            {t("settings.retry")}
          </button>
        </p>
      )}

      {!tokenSet && (
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
          placeholder={
            tokenSet ? t("settings.tokenSavedPlaceholder") : t("settings.tokenPlaceholder")
          }
          onChange={(e) => setToken(e.target.value)}
          onFocus={() => {
            tokenFocused.current = true;
          }}
          onBlur={() => {
            tokenFocused.current = false;
            // Only a token that is already stored is cleared: an edit still in
            // its quiet period stays visible until it is written.
            if (token.trim() && token.trim() === writtenToken.current) setToken("");
          }}
        />
        <span className="hint" id="token-hint">
          {tokenSet ? t("settings.tokenSavedHint") : t("settings.tokenHint")}{" "}
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
          {preview && (
            <span
              className="tray-text"
              style={
                previewColor
                  ? ({ "--cat": previewColor.light, "--cat-dark": previewColor.dark } as CSSProperties)
                  : undefined
              }
            >
              {preview}
            </span>
          )}
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
        {!isWindows && (
          <>
            <label className="check">
              <input
                type="checkbox"
                checked={tray.colorText}
                onChange={(e) => changeTray({ colorText: e.target.checked })}
              />
              <span>{t("settings.trayColorText")}</span>
            </label>
            <span className="hint">{t("settings.trayColorHint")}</span>
          </>
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
    </div>
  );
}

function open(url: string) {
  void openUrl(url).catch(() => undefined);
}
