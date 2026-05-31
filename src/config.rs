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
    pub weather: Option<WeatherConfig>,
    pub mock: Option<MockConfig>,
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

impl Config {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let config_str = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&config_str)?;
        Ok(config)
    }
}
