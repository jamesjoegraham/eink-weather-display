use include_dir::{include_dir, Dir};


static ICONS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/icons");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DayPhase {
    Day,
    Night
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WeatherCondition {
    Clear,
    PartlyCloudy,
    Cloudy,
    Rain,
    LightRain,
    LightSnow,
    Blizzard,
    Snow,
    FreezingRain,
    Hail,
    Thunderstorm,
    LightFog,
    HeavyFog,
}


impl DayPhase {
    pub fn from_time_iso(time_iso: &str) -> Self {
        let hour = time_iso
            .split_once('T')
            .and_then(|(_, hhmm)| hhmm.get(0..2))
            .and_then(|hh| hh.parse::<u8>().ok())
            .unwrap_or(12);
        if (6..18).contains(&hour) {
            DayPhase::Day
        } else {
            DayPhase::Night
        }
    }

    pub fn from_time_iso_with_sun(time_iso: &str, sunrise: &str, sunset: &str) -> Self {
        // Compare ISO strings directly (YYYY-MM-DDTHH:MM)
        if time_iso >= sunrise && time_iso < sunset {
            DayPhase::Day
        } else {
            DayPhase::Night
        }
    }
}

#[derive(Debug, Clone)]
pub enum WeatherIcon {
    UVIndex,
    Rain,
    Wind,
    Pressure,
    Humidity,
    Visibility,
}

impl WeatherIcon {
    pub fn filename(&self) -> &'static str {
        match self {
            WeatherIcon::UVIndex => "uv-index.svg",
            WeatherIcon::Rain => "raindrops.svg",
            WeatherIcon::Humidity => "raindrop.svg",
            WeatherIcon::Wind => "wind.svg",
            WeatherIcon::Pressure => "pressure.svg",
            WeatherIcon::Visibility => "visibility.svg",
        }
    }
}

impl WeatherCondition {
    pub fn from_weather_code(code: u16) -> Self {
        match code {
            0 => WeatherCondition::Clear,
            1 | 2 => WeatherCondition::PartlyCloudy,
            3 => WeatherCondition::Cloudy,
            45 => WeatherCondition::LightFog,
            48 => WeatherCondition::HeavyFog,
            51 | 53 | 61 | 80 => WeatherCondition::LightRain,
            55 | 63 | 65 | 81 | 82 => WeatherCondition::Rain,
            56 | 57 | 66 | 67 => WeatherCondition::FreezingRain,
            71 | 85 => WeatherCondition::LightSnow,
            73 | 75 | 77 => WeatherCondition::Snow,
            86 => WeatherCondition::Blizzard,
            95 => WeatherCondition::Thunderstorm,
            96 | 99 => WeatherCondition::Hail,
            _ => WeatherCondition::Cloudy,
        }
    }

    pub fn icon(&self, day_phase: &DayPhase) -> &'static str {
        let icon = match (self, day_phase) {
            (WeatherCondition::Clear, DayPhase::Day) => "sun.svg",
            (WeatherCondition::Clear, DayPhase::Night) => "moon-clear.svg",
            (WeatherCondition::PartlyCloudy, DayPhase::Day) => "partly-cloudy.svg",
            (WeatherCondition::PartlyCloudy, DayPhase::Night) => "partly-cloudy-night.svg",
            (WeatherCondition::Cloudy, DayPhase::Day) => "cloudy.svg",
            (WeatherCondition::Cloudy, DayPhase::Night) => "cloudy-night.svg",
            (WeatherCondition::Rain, _) => "rain.svg",
            (WeatherCondition::LightRain, _) => "light-rain.svg",
            (WeatherCondition::LightSnow, _) => "snow.svg",
            (WeatherCondition::Blizzard, _) => "snow.svg",
            (WeatherCondition::Snow, _) => "snow.svg",
            (WeatherCondition::FreezingRain, _) => "rain.svg",
            (WeatherCondition::Hail, _) => "rain.svg",
            (WeatherCondition::Thunderstorm, _) => "thunder.svg",
            (WeatherCondition::LightFog, _) => "fog.svg",
            (WeatherCondition::HeavyFog, _) => "fog.svg",
        };

        if ICONS_DIR.get_file(icon).is_some() {
            icon
        } else {
            "unknown.svg"
        }
    }

    pub fn summary(&self) -> &'static str {
        match self {
            WeatherCondition::Clear => "Clear",
            WeatherCondition::PartlyCloudy => "Partly cloudy",
            WeatherCondition::Cloudy => "Overcast",
            WeatherCondition::Rain => "Rain",
            WeatherCondition::LightRain => "Light rain",
            WeatherCondition::LightSnow => "Light snow",
            WeatherCondition::Blizzard => "Blizzard",
            WeatherCondition::Snow => "Snow",
            WeatherCondition::FreezingRain => "Freezing rain",
            WeatherCondition::Hail => "Hail",
            WeatherCondition::Thunderstorm => "Thunderstorm",
            WeatherCondition::LightFog => "Fog",
            WeatherCondition::HeavyFog => "Dense fog",
        }
    }
}
