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

    Ok(Snapshot {
        aqi,
        station,
        station_url,
        dominant,
        updated,
        stale: false,
        pollutants,
        attributions,
        fetched_at: now_secs(),
    })
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
            "aqi={:?} station={} pollutants={} attributions={}",
            snap.aqi,
            snap.station,
            snap.pollutants.len(),
            snap.attributions.len()
        );
        assert!(!snap.station.is_empty(), "station name should be present");
        assert!(snap.aqi.is_some(), "aqi should parse");
        assert!(!snap.attributions.is_empty(), "attributions must be present");
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
