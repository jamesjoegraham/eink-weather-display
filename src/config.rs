use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub refresh_secs: Option<u64>,
    pub theme: Option<String>,
    pub forecast_source: Option<String>,
    pub preview_svg_path: Option<String>,
    pub preview_png_path: Option<String>,
    pub preview_dithered_png_path: Option<String>,
    pub weather: Option<WeatherConfig>,
    pub mock: Option<MockConfig>,
    pub calendar: Option<CalendarConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WeatherConfig {
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub tz: Option<String>,
    pub hours: Option<usize>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MockConfig {
    pub preset: Option<String>,
    pub night: Option<bool>,
    pub hours: Option<usize>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CalendarConfig {
    /// Private ICS URL from Google Calendar Settings > [Calendar] > "Secret address in iCal format"
    pub ics_url: String,
    /// How many days ahead to fetch events (default: 7)
    pub days_ahead: Option<i64>,
    /// Max events to return (default: 50)
    pub max_events: Option<usize>,
    /// Number of day columns in the weekly grid view (default: 3)
    pub num_cols: Option<usize>,
}

impl Config {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let config_str = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&config_str)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_config() {
        let toml_str = r#"
[weather]
lat = 44.6488
lon = -63.5752
tz = "America/Halifax"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(config.refresh_secs.is_none());
        assert!(config.theme.is_none());
        let w = config.weather.unwrap();
        assert_eq!(w.lat, Some(44.6488));
        assert_eq!(w.lon, Some(-63.5752));
        assert_eq!(w.tz.as_deref(), Some("America/Halifax"));
        assert!(w.hours.is_none());
    }

    #[test]
    fn parse_full_config() {
        let toml_str = r#"
refresh_secs = 300
theme = "dark"
forecast_source = "mock"

[weather]
lat = 44.6488
lon = -63.5752
tz = "America/Halifax"
hours = 12

[mock]
preset = "thunder"
night = false
hours = 6

[calendar]
ics_url = "https://example.com/ical"
days_ahead = 14
max_events = 100
num_cols = 5
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.refresh_secs, Some(300));
        assert_eq!(config.theme.as_deref(), Some("dark"));
        assert_eq!(config.forecast_source.as_deref(), Some("mock"));

        let w = config.weather.unwrap();
        assert_eq!(w.hours, Some(12));

        let m = config.mock.unwrap();
        assert_eq!(m.preset.as_deref(), Some("thunder"));
        assert!(!m.night.unwrap());
        assert_eq!(m.hours, Some(6));

        let c = config.calendar.unwrap();
        assert_eq!(c.ics_url, "https://example.com/ical");
        assert_eq!(c.days_ahead, Some(14));
        assert_eq!(c.max_events, Some(100));
        assert_eq!(c.num_cols, Some(5));
    }

    #[test]
    fn parse_empty_config_defaults() {
        let toml_str = r#"
[weather]
lat = 44.6488
lon = -63.5752
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.refresh_secs, None);
        assert_eq!(config.theme, None);
        assert_eq!(config.forecast_source, None);
    }

    #[test]
    fn config_from_path_file_not_found() {
        let result = Config::from_path("/tmp/nonexistent-config-12345.toml");
        assert!(result.is_err());
    }
}
