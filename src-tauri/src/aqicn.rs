use crate::config::{Config, Location};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const BASE: &str = "https://api.waqi.info";

/// Pollutant codes aqicn may return, in display order.
const CODES: [&str; 10] = ["pm25", "pm10", "o3", "no2", "so2", "co", "t", "h", "p", "w"];

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("no-token")]
    NoToken,
    #[error("network-error: {0}")]
    Network(String),
    #[error("api-error: {0}")]
    Api(String),
    #[error("parse-error: {0}")]
    Parse(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pollutant {
    pub code: String,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attribution {
    pub name: String,
    pub url: Option<String>,
}

/// One day of aqicn's forecast: already expressed on the AQI scale, the same
/// one `iaqi` uses, so it can share a y axis with the station reading.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Band {
    pub avg: f64,
    pub min: f64,
    pub max: f64,
}

/// A forecast day. aqicn sends one array per pollutant; a day is present only
/// where that pollutant has a value.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForecastDay {
    /// "YYYY-MM-DD", station local date.
    pub day: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pm25: Option<Band>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pm10: Option<Band>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub o3: Option<Band>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub aqi: Option<i32>,
    pub station: String,
    pub station_url: Option<String>,
    pub dominant: Option<String>,
    /// ISO timestamp of the station reading, as sent by aqicn.
    pub updated: Option<String>,
    /// True when the cached reading is old and the last refresh failed.
    pub stale: bool,
    pub pollutants: Vec<Pollutant>,
    pub attributions: Vec<Attribution>,
    /// Unix seconds of the station reading. aqicn's own `time.v`, which is
    /// the measurement time, not the request time.
    #[serde(default)]
    pub observed_at: Option<i64>,
    /// Daily forecast from the same response, oldest first. Empty when the
    /// station publishes none.
    #[serde(default)]
    pub forecast: Vec<ForecastDay>,
    /// Unix seconds when this client fetched it.
    pub fetched_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StationHit {
    pub uid: u32,
    pub name: String,
    pub aqi: Option<i32>,
    pub country: Option<String>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
}

fn client() -> Result<reqwest::Client, ApiError> {
    reqwest::Client::builder()
        .user_agent(concat!("aqi-menubar/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(net_err)
}

fn feed_url(location: &Location) -> String {
    match location {
        Location::Here => format!("{BASE}/feed/here/"),
        Location::City { query } => format!("{BASE}/feed/{}/", urlencoding::encode(query.trim())),
        Location::Station { uid } => format!("{BASE}/feed/@{uid}/"),
        // aqicn syntax: geo:lat;lon
        Location::Geo { lat, lon } => format!("{BASE}/feed/geo:{lat};{lon}/"),
    }
}

/// A network failure with the request URL dropped. The URL carries the token,
/// and this text ends up in the panel, so it must never contain it.
fn net_err(e: reqwest::Error) -> ApiError {
    ApiError::Network(e.without_url().to_string())
}

pub async fn fetch(cfg: &Config) -> Result<Snapshot, ApiError> {
    let token = cfg.token.trim();
    if token.is_empty() {
        return Err(ApiError::NoToken);
    }
    let resp = client()?
        .get(feed_url(&cfg.location))
        .query(&[("token", token)])
        .send()
        .await
        .map_err(net_err)?;
    let body: Value = resp
        .json()
        .await
        .map_err(net_err)?;
    parse_feed(&body)
}

pub fn parse_feed(body: &Value) -> Result<Snapshot, ApiError> {
    let status = body.get("status").and_then(Value::as_str).unwrap_or("");
    if status != "ok" {
        let msg = body
            .get("data")
            .and_then(Value::as_str)
            .unwrap_or("unknown error");
        return Err(ApiError::Api(msg.to_string()));
    }
    let data = body
        .get("data")
        .filter(|v| v.is_object())
        .ok_or_else(|| ApiError::Parse("missing data object".into()))?;

    let aqi = match data.get("aqi") {
        Some(Value::Number(n)) => n.as_f64().map(|v| v.round() as i32),
        Some(Value::String(s)) => s.trim().parse::<f64>().ok().map(|v| v.round() as i32),
        _ => None,
    };

    let city = data.get("city");
    let station = city
        .and_then(|c| c.get("name"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let station_url = city
        .and_then(|c| c.get("url"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let dominant = data
        .get("dominentpol")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let updated = data
        .get("time")
        .and_then(|t| t.get("iso").or_else(|| t.get("s")))
        .and_then(Value::as_str)
        .map(str::to_string);

    let mut pollutants = Vec::new();
    if let Some(iaqi) = data.get("iaqi").and_then(Value::as_object) {
        for code in CODES {
            let value = iaqi
                .get(code)
                .and_then(|entry| entry.get("v"))
                .and_then(Value::as_f64);
            if let Some(value) = value {
                pollutants.push(Pollutant {
                    code: code.to_string(),
                    value,
                });
            }
        }
    }

    let attributions = data
        .get("attributions")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let name = item.get("name").and_then(Value::as_str)?.to_string();
                    let url = item
                        .get("url")
                        .and_then(Value::as_str)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string);
                    Some(Attribution { name, url })
                })
                .collect()
        })
        .unwrap_or_default();

    let observed_at = data
        .get("time")
        .and_then(|t| t.get("v"))
        .and_then(Value::as_i64);

    let forecast = parse_forecast(data, observed_at);

    Ok(Snapshot {
        aqi,
        station,
        station_url,
        dominant,
        updated,
        stale: false,
        pollutants,
        attributions,
        observed_at,
        forecast,
        fetched_at: now_secs(),
    })
}

/// Days a forecast is allowed to cover around the station reading. aqicn sends
/// two days back and about six forward; anything outside that is stale data
/// left in the feed (some stations carry a year-old o3 row).
const FORECAST_BACK_DAYS: i64 = 2;
const FORECAST_FORWARD_DAYS: i64 = 8;

/// Days since 1970-01-01 for a civil date (Howard Hinnant's algorithm). Enough
/// date math for the forecast window without pulling in a date crate.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn day_number(ymd: &str) -> Option<i64> {
    let mut parts = ymd.split('-');
    let y: i64 = parts.next()?.trim().parse().ok()?;
    let m: i64 = parts.next()?.trim().parse().ok()?;
    let d: i64 = parts.next()?.trim().parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some(days_from_civil(y, m, d))
}

fn band_from(item: &Value) -> Option<Band> {
    let num = |key: &str| item.get(key).and_then(Value::as_f64);
    let avg = num("avg")?;
    let min = num("min").unwrap_or(avg);
    let max = num("max").unwrap_or(avg);
    // -1 is aqicn's "no reading".
    if avg < 0.0 {
        return None;
    }
    Some(Band {
        avg,
        min: min.min(avg),
        max: max.max(avg),
    })
}

/// Merge `forecast.daily.{pm25,pm10,o3}` into one row per day, dropping days
/// outside the window around the reading.
pub fn parse_forecast(data: &Value, observed_at: Option<i64>) -> Vec<ForecastDay> {
    let Some(daily) = data.get("forecast").and_then(|f| f.get("daily")) else {
        return Vec::new();
    };
    // Without a reading time there is no window to filter by, so trust what
    // the feed sent rather than dropping everything.
    let window = observed_at.map(|t| {
        let anchor = t.div_euclid(86_400);
        (
            anchor - FORECAST_BACK_DAYS,
            anchor + FORECAST_FORWARD_DAYS,
        )
    });

    let mut days: Vec<ForecastDay> = Vec::new();
    for code in ["pm25", "pm10", "o3"] {
        let Some(rows) = daily.get(code).and_then(Value::as_array) else {
            continue;
        };
        for item in rows {
            let Some(day) = item.get("day").and_then(Value::as_str) else {
                continue;
            };
            if let Some((lo, hi)) = window {
                match day_number(day) {
                    Some(n) if n >= lo && n <= hi => {}
                    _ => continue,
                }
            }
            let Some(band) = band_from(item) else {
                continue;
            };
            let slot = match days.iter_mut().find(|d| d.day == day) {
                Some(slot) => slot,
                None => {
                    days.push(ForecastDay {
                        day: day.to_string(),
                        ..ForecastDay::default()
                    });
                    days.last_mut().unwrap()
                }
            };
            match code {
                "pm25" => slot.pm25 = Some(band),
                "pm10" => slot.pm10 = Some(band),
                _ => slot.o3 = Some(band),
            }
        }
    }

    days.sort_by(|a, b| a.day.cmp(&b.day));
    days
}

pub async fn search(cfg: &Config, keyword: &str) -> Result<Vec<StationHit>, ApiError> {
    let token = cfg.token.trim();
    if token.is_empty() {
        return Err(ApiError::NoToken);
    }
    let keyword = keyword.trim();
    if keyword.is_empty() {
        return Ok(Vec::new());
    }
    let resp = client()?
        .get(format!("{BASE}/search/"))
        .query(&[("token", token), ("keyword", keyword)])
        .send()
        .await
        .map_err(net_err)?;
    let body: Value = resp
        .json()
        .await
        .map_err(net_err)?;

    let status = body.get("status").and_then(Value::as_str).unwrap_or("");
    if status != "ok" {
        let msg = body
            .get("data")
            .and_then(Value::as_str)
            .unwrap_or("unknown error");
        return Err(ApiError::Api(msg.to_string()));
    }

    let hits = body
        .get("data")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let station = item.get("station")?;
                    let uid = item.get("uid")?.as_u64()? as u32;
                    let name = station.get("name")?.as_str()?.to_string();
                    let aqi = match item.get("aqi") {
                        Some(Value::String(s)) => s.trim().parse::<f64>().ok().map(|v| v.round() as i32),
                        Some(Value::Number(n)) => n.as_f64().map(|v| v.round() as i32),
                        _ => None,
                    };
                    let geo = station.get("geo").and_then(Value::as_array);
                    let lat = geo.and_then(|g| g.first()).and_then(Value::as_f64);
                    let lon = geo.and_then(|g| g.get(1)).and_then(Value::as_f64);
                    Some(StationHit {
                        uid,
                        name,
                        aqi,
                        country: station
                            .get("country")
                            .and_then(Value::as_str)
                            .map(str::to_string),
                        lat,
                        lon,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(hits)
}

pub fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn network_errors_do_not_leak_the_token() {
        // Nothing listens on port 9, so this fails after reqwest has already
        // attached the URL — which carries the token — to the error.
        let err = client()
            .unwrap()
            .get("http://127.0.0.1:9/feed/here/?token=SECRET-TOKEN")
            .send()
            .await
            .unwrap_err();
        let cleaned = net_err(err).to_string();
        assert!(!cleaned.contains("SECRET-TOKEN"), "{cleaned}");
        assert!(cleaned.starts_with("network-error:"), "{cleaned}");
    }

    #[test]
    fn parses_a_normal_feed() {
        let body: Value = serde_json::from_str(
            r#"{
              "status": "ok",
              "data": {
                "aqi": 50,
                "idx": 1437,
                "dominentpol": "pm25",
                "city": {"name": "Shanghai (上海)", "url": "https://aqicn.org/city/shanghai"},
                "iaqi": {"pm25": {"v": 50}, "pm10": {"v": 20}, "co": {"v": 5.5}},
                "time": {"s": "2026-11-19 10:00:00", "iso": "2026-11-19T10:00:00+08:00"},
                "attributions": [{"name": "World Air Quality Index Project", "url": "https://waqi.info/"}]
              }
            }"#,
        )
        .unwrap();
        let snap = parse_feed(&body).unwrap();
        assert_eq!(snap.aqi, Some(50));
        assert_eq!(snap.station, "Shanghai (上海)");
        assert_eq!(snap.dominant.as_deref(), Some("pm25"));
        assert_eq!(snap.pollutants.len(), 3);
        assert_eq!(snap.pollutants[0].code, "pm25");
        assert_eq!(snap.attributions.len(), 1);
        assert_eq!(snap.updated.as_deref(), Some("2026-11-19T10:00:00+08:00"));
    }

    #[test]
    fn parses_string_aqi_and_missing_fields() {
        let body: Value = serde_json::from_str(
            r#"{"status":"ok","data":{"aqi":"161","city":{"name":"Somewhere"},"iaqi":{}}}"#,
        )
        .unwrap();
        let snap = parse_feed(&body).unwrap();
        assert_eq!(snap.aqi, Some(161));
        assert!(snap.pollutants.is_empty());
        assert!(snap.station_url.is_none());
    }

    #[test]
    fn surfaces_api_error_message() {
        let body: Value =
            serde_json::from_str(r#"{"status":"error","data":"Unknown station"}"#).unwrap();
        let err = parse_feed(&body).unwrap_err();
        assert!(matches!(err, ApiError::Api(msg) if msg == "Unknown station"));
    }

    #[test]
    fn negative_aqi_is_kept_as_no_data() {
        // aqicn uses -1 for "no reading yet".
        let body: Value =
            serde_json::from_str(r#"{"status":"ok","data":{"aqi":-1,"city":{"name":"X"}}}"#).unwrap();
        let snap = parse_feed(&body).unwrap();
        assert_eq!(snap.aqi, Some(-1));
    }

    /// Hits the real API. Run with: cargo test -- --ignored live_fetch
    #[tokio::test]
    #[ignore]
    async fn live_fetch_with_demo_token() {
        let cfg = Config {
            token: "demo".to_string(),
            location: Location::Here,
            ..Config::default()
        };
        let snap = fetch(&cfg).await.expect("live fetch should succeed");
        println!(
            "aqi={:?} station={} pollutants={} attributions={} forecast_days={}",
            snap.aqi,
            snap.station,
            snap.pollutants.len(),
            snap.attributions.len(),
            snap.forecast.len()
        );
        for day in &snap.forecast {
            println!(
                "  {} pm25={:?} pm10={:?} o3={:?}",
                day.day, day.pm25, day.pm10, day.o3
            );
        }
        assert!(!snap.station.is_empty(), "station name should be present");
        assert!(snap.aqi.is_some(), "aqi should parse");
        assert!(!snap.attributions.is_empty(), "attributions must be present");
    }

    const FORECAST_BODY: &str = r#"{
      "status": "ok",
      "data": {
        "aqi": 124,
        "idx": 1437,
        "dominentpol": "pm25",
        "city": {"name": "Shanghai (上海)"},
        "iaqi": {"pm25": {"v": 124}, "pm10": {"v": 56}},
        "time": {"s": "2026-11-25 14:00:00", "v": 1795615200, "iso": "2026-11-25T14:00:00+08:00"},
        "forecast": {
          "daily": {
            "pm25": [
              {"avg": 171, "day": "2026-11-23", "max": 171, "min": 171},
              {"avg": 130, "day": "2026-11-24", "max": 171, "min": 40},
              {"avg": 221, "day": "2026-11-25", "max": 302, "min": 165}
            ],
            "pm10": [
              {"avg": 52, "day": "2026-11-24", "max": 70, "min": 12},
              {"avg": 70, "day": "2026-11-25", "max": 70, "min": 70}
            ],
            "o3": [
              {"avg": 1, "day": "2025-04-05", "max": 1, "min": 1}
            ]
          }
        }
      }
    }"#;

    #[test]
    fn the_forecast_lands_on_one_row_per_day() {
        let body: Value = serde_json::from_str(FORECAST_BODY).unwrap();
        let snap = parse_feed(&body).unwrap();
        assert_eq!(snap.observed_at, Some(1_795_615_200));
        // 11-23, 11-24, 11-25. The 2025 o3 row is outside the window.
        assert_eq!(snap.forecast.len(), 3);
        assert_eq!(snap.forecast[0].day, "2026-11-23");
        assert!(snap.forecast[0].pm10.is_none());
        let today = snap
            .forecast
            .iter()
            .find(|d| d.day == "2026-11-25")
            .expect("today");
        let pm25 = today.pm25.as_ref().expect("pm25 band");
        assert_eq!((pm25.avg, pm25.min, pm25.max), (221.0, 165.0, 302.0));
        assert!(today.o3.is_none(), "stale o3 row must not survive");
    }

    #[test]
    fn a_feed_without_forecast_is_still_fine() {
        let body: Value = serde_json::from_str(
            r#"{"status":"ok","data":{"aqi":50,"city":{"name":"X"},"iaqi":{}}}"#,
        )
        .unwrap();
        let snap = parse_feed(&body).unwrap();
        assert!(snap.forecast.is_empty());
        assert_eq!(snap.observed_at, None);
    }

    #[test]
    fn a_band_never_inverts_its_min_and_max() {
        let body: Value = serde_json::from_str(
            r#"{"status":"ok","data":{"aqi":10,"time":{"v":1795615200},
                "forecast":{"daily":{"pm25":[{"avg":50,"day":"2026-11-25","max":10,"min":90}]}}}}"#,
        )
        .unwrap();
        let snap = parse_feed(&body).unwrap();
        let band = snap.forecast[0].pm25.as_ref().unwrap();
        assert!(band.min <= band.avg && band.avg <= band.max);
    }

    #[test]
    fn builds_urls_for_each_location_kind() {
        assert_eq!(feed_url(&Location::Here), "https://api.waqi.info/feed/here/");
        assert_eq!(
            feed_url(&Location::Station { uid: 1437 }),
            "https://api.waqi.info/feed/@1437/"
        );
        assert_eq!(
            feed_url(&Location::Geo { lat: 31.2, lon: 121.4 }),
            "https://api.waqi.info/feed/geo:31.2;121.4/"
        );
        assert_eq!(
            feed_url(&Location::City {
                query: "上海 静安".into()
            }),
            "https://api.waqi.info/feed/%E4%B8%8A%E6%B5%B7%20%E9%9D%99%E5%AE%89/"
        );
    }
}
