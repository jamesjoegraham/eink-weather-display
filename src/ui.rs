use embedded_graphics::prelude::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiTheme {
    Light,
    Dark,
}

impl UiTheme {
    pub fn from_env() -> Self {
        match std::env::var("EINK_WEATHER_THEME") {
            Ok(value) if value.eq_ignore_ascii_case("dark") => UiTheme::Dark,
            _ => UiTheme::Light,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OverlayLayout {
    pub origin: Point,
    pub size: Size,
}

pub fn overlay_layout() -> OverlayLayout {
    OverlayLayout {
        origin: Point::new(620, 40),
        size: Size::new(140, 140),
    }
}
