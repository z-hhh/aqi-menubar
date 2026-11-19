import { openUrl } from "@tauri-apps/plugin-opener";
import { useTranslation } from "react-i18next";
import { categoryColor, categoryInk, categoryOf, formatTemp, readingTime, unitOf } from "./aqi";
import { bridge } from "./bridge";
import type { AppState, Pollutant } from "./types";

const MAIN_CODES = ["pm25", "pm10", "o3", "no2", "so2", "co"];

interface Props {
  state: AppState;
  onOpenSettings: () => void;
}

function open(url: string) {
  void openUrl(url).catch(() => undefined);
}

export default function Panel({ state, onOpenSettings }: Props) {
  const { t, i18n } = useTranslation();
  const snapshot = state.snapshot;
  const category = categoryOf(snapshot?.aqi ?? null);
  const value =
    snapshot?.aqi !== null && snapshot?.aqi !== undefined && snapshot.aqi >= 0
      ? String(snapshot.aqi)
      : "–";

  if (!state.config.token.trim()) {
    return (
      <div className="panel empty">
        <div className="empty-mark" aria-hidden="true">
          ◍
        </div>
        <p className="empty-title">{t("panel.noToken")}</p>
        <p className="empty-hint">{t("panel.noTokenHint")}</p>
        <button type="button" className="primary" onClick={onOpenSettings}>
          {t("settings.open")}
        </button>
      </div>
    );
  }

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
      <header className="head">
        <div
          className="badge"
          style={{ background: categoryColor[category], color: categoryInk[category] }}
        >
          <span className="badge-value">{value}</span>
          <span className="badge-label">AQI</span>
        </div>
        <div className="head-text">
          <div className="category">{t(`aqi.${category}`)}</div>
          <div className="station" title={snapshot?.station}>
            {snapshot?.station || "—"}
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
          ⟳ {t("panel.refresh")}
        </button>
        <button type="button" className="ghost" onClick={onOpenSettings}>
          ⚙ {t("panel.settings")}
        </button>
      </footer>
    </div>
  );
}
