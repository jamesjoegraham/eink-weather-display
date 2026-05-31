# E-Ink Weather Dashboard

A Rust-based weather dashboard renderer for e-ink displays, featuring:
- Live weather data from Open-Meteo API (with mock/demo fallback)
- SVG dashboard rendering using Jinja2-style templates (minijinja)
- Support for both light and dark themes
- Hourly forecast with sunrise/dusk connectors
- Compact, icon-driven metric panels
- Designed for Raspberry Pi and similar e-ink hardware

## Features
- **Live Weather:** Fetches real-time weather data, or uses mock/demo data for testing.
- **SVG Output:** Renders a visually rich dashboard as SVG, suitable for e-ink display rendering.
- **Hourly Forecast:** Shows the next 12 hours, with clear sunrise/dusk transitions.
- **Customizable UI:** Easily tweak layout, icons, and themes via Jinja2 templates.
- **Scriptable:** Includes scripts for building, deploying, and running on target hardware.

## Requirements
- Rust (stable)
- [minijinja](https://github.com/mitsuhiko/minijinja) (templating)
- [reqwest](https://docs.rs/reqwest/) (HTTP client)
- [chrono](https://docs.rs/chrono/) (date/time)
- [include_dir](https://docs.rs/include_dir/) (embed templates/icons)

## Usage

### Build
```
cargo build --release
```

### Run (on your dev machine)
```
cargo run
```

### Deploy to Pi (example)
```
scripts/deploy.sh
```

### Environment Variables
- `EINK_WEATHER_FORECAST_SOURCE` = `live` | `demo` | `mock`
- `EINK_WEATHER_LAT`, `EINK_WEATHER_LON`, `EINK_WEATHER_TZ` (for live)
- `EINK_WEATHER_MOCK_PRESET` (for mock)

## Project Structure
- `src/` — Rust source code
  - `main.rs` — Entry point
  - `template.rs` — SVG rendering logic
  - `weather.rs` — Data models & API
  - `display.rs` — Icon/phase logic
  - `ui.rs` — UI helpers
- `templates/` — Jinja2 SVG templates
- `icons/` — SVG icons
- `fonts/` — Font files
- `scripts/` — Build/deploy scripts

## Customization
- Edit SVG templates in `templates/` for layout/theme changes.
- Add or replace icons in `icons/`.
- Adjust metric panel layout in `src/template.rs`.

## License
MIT

---

*Created by James Graham, 2023–2026.*
