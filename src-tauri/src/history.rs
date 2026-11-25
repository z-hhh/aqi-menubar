//! Local record of the readings this client has seen.
//!
//! aqicn has no history endpoint, so the only in-day curve the app can draw is
//! one it keeps itself. Every poll appends one point, keyed by the station's
//! measurement time rather than the fetch time: a station reading is often an
//! hour old, and keying on "when we asked" would slide every point right.
use crate::aqicn::Snapshot;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// A week is enough to show "yesterday" and to survive a closed laptop.
const RETAIN_SECS: i64 = 7 * 86_400;
/// Hard ceiling on stored points, whatever the refresh interval is.
const MAX_POINTS: usize = 2_000;
/// Points handed to the UI. Anything denser than this is invisible in a
/// 300 px wide chart.
const MAX_UI_POINTS: usize = 400;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Obs {
    pub station: String,
    /// Unix seconds of the measurement itself.
    pub observed_at: i64,
    pub aqi: Option<i32>,
    pub pm25: Option<f64>,
}

#[derive(Debug, Default)]
pub struct History {
    path: Option<PathBuf>,
    items: Vec<Obs>,
}

fn pm25_of(snapshot: &Snapshot) -> Option<f64> {
    snapshot
        .pollutants
        .iter()
        .find(|p| p.code == "pm25")
        .map(|p| p.value)
}

impl History {
    fn path(app: &AppHandle) -> Option<PathBuf> {
        app.path()
            .app_data_dir()
            .ok()
            .map(|dir| dir.join("history.json"))
    }

    /// Never fails: an unreadable file just starts an empty history.
    pub fn load(app: &AppHandle) -> Self {
        let path = Self::path(app);
        let items = path
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|raw| serde_json::from_str::<Vec<Obs>>(&raw).ok())
            .unwrap_or_default();
        let mut history = Self { path, items };
        history.compact(crate::aqicn::now_secs());
        history
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Append one reading. Returns whether anything changed, so the caller can
    /// skip an unnecessary write.
    pub fn record(&mut self, snapshot: &Snapshot) -> bool {
        // A reading with no station name or time cannot be placed on a curve.
        if snapshot.station.trim().is_empty() {
            return false;
        }
        let observed_at = snapshot.observed_at.unwrap_or(snapshot.fetched_at);
        let obs = Obs {
            station: snapshot.station.clone(),
            observed_at,
            aqi: snapshot.aqi.filter(|v| *v >= 0),
            pm25: pm25_of(snapshot),
        };

        match self
            .items
            .iter_mut()
            .find(|o| o.station == obs.station && o.observed_at == obs.observed_at)
        {
            // The same measurement refreshed: the newer payload wins, because
            // a station sometimes publishes a provisional value first.
            Some(slot) => {
                if *slot == obs {
                    return false;
                }
                *slot = obs;
            }
            None => self.items.push(obs),
        }

        self.items.sort_by_key(|o| o.observed_at);
        self.compact(crate::aqicn::now_secs());
        self.save();
        true
    }

    /// Drop what is too old and keep only the newest `MAX_POINTS`.
    fn compact(&mut self, now: i64) {
        let cutoff = now - RETAIN_SECS;
        self.items.retain(|o| o.observed_at >= cutoff);
        if self.items.len() > MAX_POINTS {
            let drop = self.items.len() - MAX_POINTS;
            self.items.drain(0..drop);
        }
    }

    fn save(&self) {
        let Some(path) = self.path.as_ref() else {
            return;
        };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_string(&self.items) {
            let _ = std::fs::write(path, json);
        }
    }

    /// The points that belong to the station currently on screen. Older
    /// stations stay on disk but are never mixed into this line.
    pub fn for_station(&self, station: &str) -> Vec<Obs> {
        let mut out: Vec<Obs> = self
            .items
            .iter()
            .filter(|o| o.station == station && o.aqi.is_some())
            .cloned()
            .collect();
        if out.len() > MAX_UI_POINTS {
            let step = out.len().div_ceil(MAX_UI_POINTS);
            let last = out.len() - 1;
            out = out
                .iter()
                .enumerate()
                .filter(|(i, _)| i % step == 0 || *i == last)
                .map(|(_, o)| o.clone())
                .collect();
        }
        out
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.save();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Timestamps are always relative to "now", because `record` prunes
    /// anything older than the retention window.
    fn base() -> i64 {
        crate::aqicn::now_secs() - 7_200
    }

    fn snap(station: &str, observed_at: i64, aqi: i32) -> Snapshot {
        Snapshot {
            aqi: Some(aqi),
            station: station.to_string(),
            station_url: None,
            dominant: Some("pm25".into()),
            updated: None,
            stale: false,
            pollutants: vec![crate::aqicn::Pollutant {
                code: "pm25".into(),
                value: 42.0,
            }],
            attributions: Vec::new(),
            observed_at: Some(observed_at),
            forecast: Vec::new(),
            fetched_at: observed_at + 60,
        }
    }

    #[test]
    fn a_repeated_measurement_does_not_add_a_second_point() {
        let mut h = History::default();
        let a = snap("S", base(), 50);
        assert!(h.record(&a));
        // Same measurement, refreshed: no new point.
        assert!(!h.record(&a));
        assert_eq!(h.items.len(), 1);
        // A newer measurement does.
        assert!(h.record(&snap("S", base() + 3_600, 61)));
        assert_eq!(h.items.len(), 2);
        assert_eq!(h.items[1].aqi, Some(61));
    }

    #[test]
    fn a_refreshed_measurement_takes_the_newer_value() {
        let mut h = History::default();
        h.record(&snap("S", base(), 50));
        h.record(&snap("S", base(), 77));
        assert_eq!(h.items.len(), 1);
        assert_eq!(h.items[0].aqi, Some(77));
    }

    #[test]
    fn the_measurement_time_wins_over_the_fetch_time() {
        let mut h = History::default();
        // Station time is an hour behind the fetch, and must not slide.
        let mut s = snap("S", base(), 50);
        s.fetched_at = base() + 3_600;
        h.record(&s);
        assert_eq!(h.items[0].observed_at, base());
    }

    #[test]
    fn stations_are_never_mixed_on_one_curve() {
        let mut h = History::default();
        h.record(&snap("Shanghai", base(), 50));
        h.record(&snap("Beijing", base() + 3_600, 150));
        h.record(&snap("Shanghai", base() + 7_200, 55));
        let points = h.for_station("Shanghai");
        assert_eq!(points.len(), 2);
        assert!(points.iter().all(|p| p.station == "Shanghai"));
    }

    #[test]
    fn readings_without_a_value_are_not_drawn() {
        let mut h = History::default();
        let mut s = snap("S", base(), 50);
        s.aqi = Some(-1);
        h.record(&s);
        h.record(&snap("S", base() + 3_600, 60));
        assert_eq!(h.for_station("S").len(), 1);
    }

    #[test]
    fn old_points_are_dropped() {
        let mut h = History::default();
        let now = crate::aqicn::now_secs();
        h.items = vec![Obs {
            station: "S".into(),
            observed_at: now - RETAIN_SECS - 60,
            aqi: Some(1),
            pm25: None,
        }];
        h.compact(now);
        assert!(h.items.is_empty());
    }

    #[test]
    fn a_long_history_is_thinned_but_keeps_the_newest_point() {
        let mut h = History::default();
        for i in 0..1_000 {
            h.items.push(Obs {
                station: "S".into(),
                observed_at: 1_700_000_000 + i * 60,
                aqi: Some(i as i32),
                pm25: None,
            });
        }
        let points = h.for_station("S");
        assert!(points.len() <= MAX_UI_POINTS + 1);
        assert_eq!(points.last().unwrap().aqi, Some(999));
    }
}
