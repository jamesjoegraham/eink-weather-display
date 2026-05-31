use embedded_graphics::prelude::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiTheme {
    Light,
    Dark,
}

use std::str::FromStr;
impl std::str::FromStr for UiTheme {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "dark" => Ok(UiTheme::Dark),
            _ => Ok(UiTheme::Light),
        }
    }
}

impl Default for UiTheme {
    fn default() -> Self {
        UiTheme::Light
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
