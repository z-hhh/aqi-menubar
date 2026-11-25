import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { categoryColor, categoryOf } from "./aqi";
import type { AppState, Band, ForecastDay } from "./types";

/**
 * The in-day curve plus the daily forecast, on one axis.
 *
 * Both series are AQI (aqicn converts its forecast from ug/m3 with the same
 * EPA table it uses for `iaqi`), so a single y axis is honest. What is not
 * honest is drawing them the same way: the forecast is a daily aggregate, not
 * a reading, so it gets a faint min-max band and a dashed daily mean, while
 * the observed line stays thin and leaves gaps where the app was not running.
 */

const HOUR = 3_600_000;
const DAY = 24 * HOUR;

const W = 312;
const PLOT_TOP = 4;
const PLOT_H = 84;
const PAD_L = 22;
const PAD_R = 5;
const AXIS_H = 13;
const H = PLOT_TOP + PLOT_H + AXIS_H;

/** AQI band edges, for the faint background stripes. */
const EDGES = [50, 100, 150, 200, 300];
const EDGE_NAMES = ["good", "moderate", "sensitive", "unhealthy", "veryUnhealthy"] as const;

interface Point {
  t: number;
  v: number;
}

function midnightOf(ts: number): number {
  const d = new Date(ts);
  d.setHours(0, 0, 0, 0);
  return d.getTime();
}

function midnightOfDay(day: string): number {
  return new Date(`${day}T00:00:00`).getTime();
}

function bandOf(day: ForecastDay, code: string): Band | undefined {
  if (code === "pm25") return day.pm25;
  if (code === "pm10") return day.pm10;
  if (code === "o3") return day.o3;
  return undefined;
}

/** The chart follows the dominant pollutant when aqicn forecasts it. */
function pickSeries(forecast: ForecastDay[], dominant: string | null) {
  const order = [dominant ?? "", "pm25", "pm10", "o3"].filter(Boolean);
  for (const code of order) {
    const days = forecast
      .map((d) => ({ t: midnightOfDay(d.day), band: bandOf(d, code) }))
      .filter((d): d is { t: number; band: Band } => Boolean(d.band) && Number.isFinite(d.t));
    if (days.length > 0) return { code, days };
  }
  return null;
}

function niceMax(value: number): number {
  for (const step of [50, 100, 150, 200, 250, 300, 400, 500]) {
    if (value <= step) return step;
  }
  return Math.ceil(value / 100) * 100;
}

/** Break the line where the app was closed, instead of drawing over the gap. */
function segments(points: Point[], gapLimit: number): Point[][] {
  const out: Point[][] = [];
  let run: Point[] = [];
  for (const point of points) {
    const previous = run[run.length - 1];
    if (previous && point.t - previous.t > gapLimit) {
      if (run.length > 0) out.push(run);
      run = [];
    }
    run.push(point);
  }
  if (run.length > 0) out.push(run);
  return out;
}

export default function TrendChart({ state }: { state: AppState }) {
  const { t, i18n } = useTranslation();
  const snapshot = state.snapshot;

  const model = useMemo(() => {
    const observed: Point[] = state.history
      .filter((o) => typeof o.aqi === "number" && (o.aqi as number) >= 0)
      .map((o) => ({ t: o.observedAt * 1000, v: o.aqi as number }))
      .sort((a, b) => a.t - b.t);

    const series = pickSeries(snapshot?.forecast ?? [], snapshot?.dominant ?? null);
    if (observed.length < 2 && !series) return null;

    const now = Date.now();
    const today = midnightOf(now);
    // A fixed four-day window: yesterday through the day after tomorrow. Any
    // wider and a week of forecast squeezes the day curve into a sliver; any
    // narrower and "yesterday" does not fit.
    const start = today - DAY;
    let end = today + 3 * DAY;
    const lastObs = observed[observed.length - 1];
    if (lastObs && lastObs.t + 2 * HOUR > end) end = lastObs.t + 2 * HOUR;

    // Only today and later: the past is the cache's job, and drawing a model
    // value next to a measurement for the same day invites false comparisons.
    const forecast = (series?.days ?? []).filter((d) => d.t >= today && d.t < end);
    if (forecast.length === 0 && observed.length < 2) return null;

    const values = [
      ...observed.filter((p) => p.t >= start - 2 * HOUR && p.t <= end).map((p) => p.v),
      ...forecast.flatMap((d) => [d.band.min, d.band.max]),
    ];
    if (values.length === 0) return null;

    return {
      observed,
      forecast,
      start,
      end,
      now,
      yMax: niceMax(Math.max(...values) * 1.08),
      code: series?.code ?? null,
    };
  }, [state.history, snapshot]);

  if (!model) return null;

  const { observed, forecast, start, end, now, yMax, code } = model;
  const plotW = W - PAD_L - PAD_R;
  const x = (ts: number) => PAD_L + ((ts - start) / (end - start)) * plotW;
  const y = (value: number) => PLOT_TOP + PLOT_H - (Math.min(value, yMax) / yMax) * PLOT_H;

  const gapLimit = Math.max(3 * state.config.refreshMinutes * 60_000, 90 * 60_000);
  const runs = segments(
    observed.filter((p) => p.t >= start && p.t <= end),
    gapLimit,
  );

  const dayFormat = new Intl.DateTimeFormat(i18n.language, { month: "numeric", day: "numeric" });
  const ticks: number[] = [];
  for (let day = start; day < end; day += DAY) ticks.push(day);

  const last = observed[observed.length - 1];
  const gridlines = [EDGES[1], EDGES[3]].filter((value) => value < yMax);
  const forecastLabel = code ? t(`pollutants.${code}`) : "";

  return (
    <section className="chart" aria-label={t("chart.title")}>
      <div className="chart-legend">
        <span className="chart-key">
          <span className="chart-swatch line" aria-hidden="true" />
          {t("chart.observed")}
        </span>
        {forecast.length > 0 && (
          <span className="chart-key">
            <span className="chart-swatch band" aria-hidden="true" />
            {t("chart.forecast")} · {forecastLabel}
          </span>
        )}
      </div>

      <svg
        className="chart-svg"
        viewBox={`0 0 ${W} ${H}`}
        width="100%"
        height={H}
        role="img"
        aria-label={`${t("chart.observed")} ${observed.length} · ${t("chart.forecast")} ${forecast.length}`}
      >
        <defs>
          <clipPath id="chart-plot">
            <rect x={PAD_L} y={PLOT_TOP - 2} width={plotW} height={PLOT_H + 4} />
          </clipPath>
          {/* The forecast only covers what has not happened yet, so today's
              band never sits on top of this morning's measured line. */}
          <clipPath id="chart-future">
            <rect
              x={x(now)}
              y={PLOT_TOP - 2}
              width={Math.max(0, W - PAD_R - x(now))}
              height={PLOT_H + 4}
            />
          </clipPath>
        </defs>

        {/* AQI bands as very faint stripes: the shape of the curve reads
            without reading the numbers. */}
        {EDGES.map((edge, i) => {
          const lower = i === 0 ? 0 : EDGES[i - 1];
          if (lower >= yMax) return null;
          const top = Math.min(edge, yMax);
          return (
            <rect
              key={edge}
              x={PAD_L}
              y={y(top)}
              width={plotW}
              height={Math.max(0, y(lower) - y(top))}
              fill={categoryColor[EDGE_NAMES[i]]}
              opacity={0.07}
            />
          );
        })}

        {gridlines.map((value) => (
          <line
            key={value}
            x1={PAD_L}
            x2={W - PAD_R}
            y1={y(value)}
            y2={y(value)}
            className="chart-grid"
          />
        ))}

        <g clipPath="url(#chart-plot)">
          <g clipPath="url(#chart-future)">
          {forecast.map((d) => {
            const x0 = Math.max(x(d.t), PAD_L);
            const x1 = Math.min(x(d.t + DAY), W - PAD_R);
            return (
              <rect
                key={d.t}
                className="chart-band"
                x={x0}
                y={y(d.band.max)}
                width={Math.max(0, x1 - x0)}
                height={Math.max(1, y(d.band.min) - y(d.band.max))}
              />
            );
          })}

          {forecast.length > 1 && (
            <polyline
              className="chart-forecast-line"
              points={forecast
                .map((d) => `${x(d.t + DAY / 2).toFixed(1)},${y(d.band.avg).toFixed(1)}`)
                .join(" ")}
            />
          )}
          </g>

          <line className="chart-now" x1={x(now)} x2={x(now)} y1={PLOT_TOP} y2={PLOT_TOP + PLOT_H} />

          {runs.map((run) =>
            run.length === 1 ? (
              <circle
                key={run[0].t}
                className="chart-dot"
                cx={x(run[0].t)}
                cy={y(run[0].v)}
                r={1.6}
              />
            ) : (
              <polyline
                key={run[0].t}
                className="chart-line"
                points={run.map((p) => `${x(p.t).toFixed(1)},${y(p.v).toFixed(1)}`).join(" ")}
              />
            ),
          )}

          {last && last.t >= start && last.t <= end && (
            <circle
              className="chart-current"
              cx={x(last.t)}
              cy={y(last.v)}
              r={3.2}
              fill={categoryColor[categoryOf(last.v)]}
            />
          )}
        </g>

        {gridlines.map((value) => (
          <text key={`l${value}`} className="chart-ylabel" x={PAD_L - 4} y={y(value) + 3}>
            {value}
          </text>
        ))}

        {ticks.map((day) => {
          const centre = x(day + DAY / 2);
          if (centre < PAD_L + 8 || centre > W - PAD_R - 8) return null;
          return (
            <text key={`d${day}`} className="chart-xlabel" x={centre} y={H - 2}>
              {dayFormat.format(day)}
            </text>
          );
        })}
      </svg>
    </section>
  );
}
