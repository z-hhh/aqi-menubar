//! One place for the AQI bands, so the tray dot colour, the level word and the
//! panel category can never drift apart.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Good,
    Moderate,
    Sensitive,
    Unhealthy,
    VeryUnhealthy,
    Hazardous,
    Unknown,
}

impl Level {
    /// US EPA bands, the ones aqicn leads with. Same numeric edges as the
    /// Chinese AQI standard, which is why the Chinese words below apply.
    pub fn of(aqi: Option<i32>) -> Self {
        let aqi = match aqi {
            Some(v) if v >= 0 => v,
            _ => return Level::Unknown,
        };
        match aqi {
            0..=50 => Level::Good,
            51..=100 => Level::Moderate,
            101..=150 => Level::Sensitive,
            151..=200 => Level::Unhealthy,
            201..=300 => Level::VeryUnhealthy,
            _ => Level::Hazardous,
        }
    }

    /// True for every real band; `Unknown` means aqicn had no reading.
    pub fn is_known(self) -> bool {
        self != Level::Unknown
    }

    /// The menu bar has room for a word, not a sentence: the panel carries the
    /// long form from the locale files, this is what fits beside a number.
    pub fn short(self, lang: &str) -> &'static str {
        match lang {
            "zh-CN" => match self {
                Level::Good => "优",
                Level::Moderate => "良",
                Level::Sensitive => "轻度污染",
                Level::Unhealthy => "中度污染",
                Level::VeryUnhealthy => "重度污染",
                Level::Hazardous => "严重污染",
                Level::Unknown => "–",
            },
            "ja" => match self {
                Level::Good => "良好",
                Level::Moderate => "普通",
                Level::Sensitive => "要注意",
                Level::Unhealthy => "不健康",
                Level::VeryUnhealthy => "非常に悪い",
                Level::Hazardous => "危険",
                Level::Unknown => "–",
            },
            _ => match self {
                Level::Good => "Good",
                Level::Moderate => "Moderate",
                Level::Sensitive => "Sensitive",
                Level::Unhealthy => "Unhealthy",
                Level::VeryUnhealthy => "Very unhealthy",
                Level::Hazardous => "Hazardous",
                Level::Unknown => "–",
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bands_follow_the_standard_edges() {
        assert_eq!(Level::of(None), Level::Unknown);
        assert_eq!(Level::of(Some(-3)), Level::Unknown);
        assert_eq!(Level::of(Some(0)), Level::Good);
        assert_eq!(Level::of(Some(50)), Level::Good);
        assert_eq!(Level::of(Some(51)), Level::Moderate);
        assert_eq!(Level::of(Some(100)), Level::Moderate);
        assert_eq!(Level::of(Some(101)), Level::Sensitive);
        assert_eq!(Level::of(Some(150)), Level::Sensitive);
        assert_eq!(Level::of(Some(151)), Level::Unhealthy);
        assert_eq!(Level::of(Some(200)), Level::Unhealthy);
        assert_eq!(Level::of(Some(201)), Level::VeryUnhealthy);
        assert_eq!(Level::of(Some(300)), Level::VeryUnhealthy);
        assert_eq!(Level::of(Some(301)), Level::Hazardous);
        assert_eq!(Level::of(Some(500)), Level::Hazardous);
    }

    #[test]
    fn short_words_are_localised() {
        assert_eq!(Level::of(Some(51)).short("zh-CN"), "良");
        assert_eq!(Level::of(Some(51)).short("ja"), "普通");
        assert_eq!(Level::of(Some(51)).short("en"), "Moderate");
        // Unknown language falls back to English rather than an empty string.
        assert_eq!(Level::of(Some(180)).short("fr"), "Unhealthy");
        assert_eq!(Level::Unknown.short("zh-CN"), "–");
    }
}
