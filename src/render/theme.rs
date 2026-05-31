
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiTheme {
    Light,
    Dark,
}

use std::str::FromStr;
impl FromStr for UiTheme {
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
