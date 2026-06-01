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
