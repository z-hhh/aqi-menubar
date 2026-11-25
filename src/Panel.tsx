import { useState, type CSSProperties } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useTranslation } from "react-i18next";
import {
  categoryColor,
  categoryInk,
  categoryOf,
  categoryText,
  formatTemp,
  readingTime,
  stationDisplay,
  unitOf,
} from "./aqi";
import { bridge } from "./bridge";
import TrendChart from "./TrendChart";
import type { AppState, Pollutant } from "./types";

const MAIN_CODES = ["pm25", "pm10", "o3", "no2", "so2", "co"];
export const TOKEN_URL = "https://aqicn.org/data-platform/token/";

interface Props {
  state: AppState;
  onOpenSettings: (focusToken?: boolean) => void;
}

/**
 * The footer used to draw "⟳" and "⚙" in the running text, which renders at
 * roughly 9 px next to 12 px labels. These are drawn at a fixed 13 px so they
 * line up with the text and keep their weight on both platforms.
 */
function RefreshIcon() {
  return (
    <svg className="btn-icon" viewBox="0 0 16 16" width="13" height="13" aria-hidden="true">
      <path
        d="M13.4 8a5.4 5.4 0 1 1-1.6-3.83"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
      />
      <path
        d="M13.5 2.4v3.7h-3.7"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function SettingsIcon() {
  return (
    <svg className="btn-icon" viewBox="0 0 16 16" width="13" height="13" aria-hidden="true">
      <path d="M2 4.5h7M12.6 4.5H14M2 11.5h1.4M7 11.5h7" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
      <circle cx="10.8" cy="4.5" r="1.7" fill="none" stroke="currentColor" strokeWidth="1.5" />
      <circle cx="5.2" cy="11.5" r="1.7" fill="none" stroke="currentColor" strokeWidth="1.5" />
    </svg>
  );
}

export function open(url: string) {
  void openUrl(url).catch(() => undefined);
}

function Onboarding({ onOpenSettings }: { onOpenSettings: (focusToken?: boolean) => void }) {
  const { t } = useTranslation();
  return (
    <div className="panel onboarding">
      <div className="onboard-head">
        <span className="onboard-mark" aria-hidden="true">
          ◍
        </span>
        <div>
          <h1 className="empty-title">{t("onboard.title")}</h1>
          <p className="empty-hint">{t("onboard.subtitle")}</p>
        </div>
      </div>

      <ol className="steps">
        <li>
          <span className="step-n" aria-hidden="true">
            1
          </span>
          <span className="step-text">{t("onboard.step1")}</span>
        </li>
        <li>
          <span className="step-n" aria-hidden="true">
            2
          </span>
          <span className="step-text">{t("onboard.step2")}</span>
        </li>
        <li>
          <span className="step-n" aria-hidden="true">
            3
          </span>
          <span className="step-text">{t("onboard.step3")}</span>
        </li>
      </ol>

      <button type="button" className="primary wide" onClick={() => open(TOKEN_URL)}>
        {t("onboard.getToken")} ↗
      </button>
      <button type="button" className="ghost wide" onClick={() => onOpenSettings(true)}>
        {t("onboard.haveToken")}
      </button>
      <p className="hint center">{t("onboard.footnote")}</p>
    </div>
  );
}

function UpdateBanner({ state }: { state: AppState }) {
  const { t } = useTranslation();
  const [dismissed, setDismissed] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const update = state.update;
  if (!update || dismissed) return null;

  return (
    <div className="update">
      <div className="update-text">
        <strong>{t("update.available", { version: update.version })}</strong>
        <span className="update-note">{t("update.note")}</span>
      </div>
      <div className="update-actions">
        <button
          type="button"
          className="primary small"
          disabled={state.installing}
          onClick={() => {
            setError(null);
            void bridge.installUpdate().catch((err) => setError(String(err)));
          }}
        >
          {state.installing ? t("update.installing") : t("update.install")}
        </button>
        <button
          type="button"
          className="ghost small"
          aria-label={t("update.later")}
          onClick={() => setDismissed(true)}
        >
          ✕
        </button>
      </div>
      {error && <p className="error">{t("update.failed", { error })}</p>}
    </div>
  );
}

export default function Panel({ state, onOpenSettings }: Props) {
  const { t, i18n } = useTranslation();
  const snapshot = state.snapshot;

  if (!state.tokenSet) {
    return <Onboarding onOpenSettings={onOpenSettings} />;
  }

  const category = categoryOf(snapshot?.aqi ?? null);
  const value =
    snapshot?.aqi !== null && snapshot?.aqi !== undefined && snapshot.aqi >= 0
      ? String(snapshot.aqi)
      : "–";

  const pollutant = (code: string): Pollutant | undefined =>
    snapshot?.pollutants.find((p) => p.code === code);
  const visible = MAIN_CODES.map(pollutant).filter((p): p is Pollutant => Boolean(p));
  const temp = pollutant("t");
  const humidity = pollutant("h");
  const wind = pollutant("w");
  const dominant = snapshot?.dominant ? pollutant(snapshot.dominant) : undefined;
  const failed = Boolean(state.error) && state.error !== "no-token";

  return (
    <div className="panel">
      <UpdateBanner state={state} />

      <header className="head">
        <div
          className="badge"
          style={{ background: categoryColor[category], color: categoryInk[category] }}
        >
          <span className="badge-value">{value}</span>
          <span className="badge-label">AQI</span>
        </div>
        <div className="head-text">
          <div
            className={state.config.tray.colorText ? "category colored" : "category"}
            style={
              state.config.tray.colorText
                ? ({
                    "--cat": categoryText[category].light,
                    "--cat-dark": categoryText[category].dark,
                  } as CSSProperties)
                : undefined
            }
          >
            {t(`aqi.${category}`)}
          </div>
          <div className="station" title={snapshot?.station}>
            {stationDisplay(snapshot?.station, state.resolvedLanguage) || "—"}
          </div>
          <div className="updated">
            {snapshot?.updated
              ? t("panel.updated", { time: readingTime(snapshot, i18n.language) })
              : t("panel.never")}
          </div>
        </div>
      </header>

      {failed && (
        <p className="notice">
          {t("panel.stale")}
          <span className="notice-detail"> {state.error}</span>
        </p>
      )}

      {visible.length > 0 && (
        <section className="grid">
          {visible.map((p) => (
            <div className="cell" key={p.code}>
              <span className="cell-name">{t(`pollutants.${p.code}`)}</span>
              <span className="cell-value">
                {Math.round(p.value * 10) / 10}
                <span className="cell-unit">{unitOf(p.code)}</span>
              </span>
            </div>
          ))}
        </section>
      )}

      <section className="facts">
        <span>
          {t("panel.dominant")}: <strong>{dominant ? t(`pollutants.${dominant.code}`) : "—"}</strong>
        </span>
        {temp && <span>{formatTemp(temp.value, "c")}</span>}
        {humidity && <span>{Math.round(humidity.value)}%</span>}
        {wind && <span>{wind.value} m/s</span>}
      </section>

      <TrendChart state={state} />

      <section className="sources">
        <div className="sources-title">{t("panel.sources")}</div>
        {snapshot?.attributions.map((item) =>
          item.url ? (
            <button
              key={item.name}
              type="button"
              className="link"
              onClick={() => open(item.url as string)}
            >
              {item.name}
            </button>
          ) : (
            <span key={item.name} className="source-name">
              {item.name}
            </span>
          ),
        )}
      </section>

      <footer className="foot">
        <button
          type="button"
          className="ghost"
          onClick={() => void bridge.refreshNow()}
          title={t("panel.refresh")}
        >
          <RefreshIcon /> {t("panel.refresh")}
        </button>
        <button type="button" className="ghost" onClick={() => onOpenSettings()}>
          <SettingsIcon /> {t("panel.settings")}
        </button>
      </footer>
    </div>
  );
}
