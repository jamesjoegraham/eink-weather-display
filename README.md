# E-Ink Weather Dashboard

A Rust-based weather dashboard renderer for **Waveshare 7.3″ ACeP (7-colour) e-ink displays**, designed for Raspberry Pi and similar hardware. Renders live weather data from [Open-Meteo](https://open-meteo.com/) as a rich SVG dashboard, then rasterizes and dithers to the 7-colour e-ink palette.

Supports both **weather** and **calendar** panels, with a Jinja2-style templating system (minijinja) for layout customisation.

## Features

- **Live weather** — fetches real-time forecast from Open-Meteo (free, no API key)
- **Mock/demo modes** — develop and test without hitting the API
- **SVG render pipeline** — Jinja2 templates → SVG → rasterized PNG → dithered e-ink frame
- **Hourly forecast** — up to 12 hours with sunrise/dusk connectors
- **Calendar panel** — weekly grid via Google Calendar ICS feed
- **Light & dark themes** — swap at runtime
- **Preview mode** — test rendering on Mac/Linux without hardware
- **Hardware support** — Waveshare 7.3″ 7-colour e-paper (SPI, GPIO)

## Project Structure

```
├── src/
│   ├── main.rs              # Entry point, CLI args, run loop
│   ├── config.rs            # TOML config loading & types
│   ├── api/
│   │   ├── weather.rs       # Open-Meteo API, demo/mock generators
│   │   └── gcalendar.rs     # ICS feed parser + RRULE expansion
│   ├── model/
│   │   ├── weather.rs       # WeatherCondition, WeatherIcon, DayPhase enums
│   │   └── calendar.rs      # CalendarEventModel, CalendarDayModel
│   ├── panel/
│   │   ├── weather.rs       # WeatherPanelViewModel (data → template context)
│   │   └── calendar.rs      # CalendarPanelViewModel (data → template context)
│   └── render/
│       ├── template.rs      # Minijinja environment, icon embedding helpers
│       ├── rasterize.rs     # SVG → PNG, supersampling + Bayer dithering
│       ├── theme.rs         # UiTheme enum (Light/Dark)
│       └── pi.rs            # Linux-only: SPI, GPIO, Waveshare EPD driver
├── templates/               # Jinja2 SVG templates (*.svg.j2)
├── icons/                   # SVG weather icons
├── fonts/                   # Embedded fonts (Roboto, Roboto Mono)
├── scripts/                 # Build/deploy/run helpers for Raspberry Pi
│   ├── build_pi.sh          # Cross-compile for Pi (zigbuild, cross, native)
│   ├── deploy.sh            # SCP binary to Pi
│   ├── run_pi.sh            # SSH into Pi and run
│   ├── generate_mock_suite.sh # Render all mock presets for visual comparison
│   └── common.sh            # Shared SSH/SCP helpers
├── .env.sample              # Environment variable reference
└── Cargo.toml
```

## Quick Start

### Prerequisites

- Rust 2024 edition (stable)
- A [config.toml](#configuration) (default path: `./config.toml`)

### Build

```bash
cargo build --release
```

### Run (desktop preview)

One-shot mode (render and exit):

```bash
EINK_WEATHER_PREVIEW_SVG_PATH=/tmp/eink-preview.svg \
EINK_WEATHER_PREVIEW_PNG_PATH=/tmp/eink-preview.png \
cargo run
```

Refreshing loop (every 5 minutes):

```bash
cargo run
# Set refresh_secs in config.toml, or EINK_WEATHER_REFRESH_SECS=300 env var
```

### Run with demo/mock data (no API key needed)

```bash
# Built-in static demo
EINK_WEATHER_FORECAST_SOURCE=demo EINK_WEATHER_REFRESH_SECS=0 cargo run

# Configurable mock (thunderstorm, clear, snow, etc.)
EINK_WEATHER_FORECAST_SOURCE=mock \
  EINK_WEATHER_MOCK_PRESET=thunder \
  EINK_WEATHER_REFRESH_SECS=0 \
  cargo run
```

### Generate full mock visual suite

```bash
scripts/generate_mock_suite.sh /tmp/eink-mock-suite
# Produces 22 files (11 presets × SVG + PNG) for visual comparison
```

## Configuration

Configuration is via **TOML** (default: `./config.toml`). Environment variables override specific values.

### Minimal example

```toml
[weather]
lat = 44.6488
lon = -63.5752
tz = "America/Halifax"
hours = 12
```

### Full config reference

```toml
# ── General ──
refresh_secs = 300         # Refresh loop interval (0 = one-shot; min = 60)
theme = "light"            # "light" or "dark"
forecast_source = "live"   # "live" (Open-Meteo), "demo", or "mock"

# ── Preview paths (desktop development) ──
preview_svg_path = "/tmp/eink-preview.svg"
preview_png_path = "/tmp/eink-preview.png"
preview_dithered_png_path = "/tmp/eink-preview-dithered.png"

# ── Weather panel ──
[weather]
lat = 44.6488             # Latitude (Open-Meteo)
lon = -63.5752            # Longitude
tz = "America/Halifax"    # Timezone for display
hours = 12                # Forecast hours to fetch (2–48)

# ── Mock source (only when forecast_source = "mock") ──
[mock]
preset = "thunder"        # clear, partly-cloudy, cloudy, fog, drizzle, rain,
                          # showers, snow, snow-showers, thunder, hail-thunder
night = false             # Render night-phase icons
hours = 12                # Mock hours to generate

# ── Calendar panel (use --panel calendar) ──
[calendar]
ics_url = "https://calendar.google.com/calendar/ical/..."
days_ahead = 7            # Days to fetch ahead (default: 7)
max_events = 50           # Max events to show (default: 50)
num_cols = 3              # Day columns in grid (default: 3)
```

### Environment variables

See [.env.sample](./.env.sample) for all supported env vars.

## Usage

### Panels

Two built-in panels, selected via `--panel`:

```bash
# Weather panel (default)
cargo run

# Calendar panel (requires [calendar] config)
cargo run -- --panel calendar
```

### Deploy to Raspberry Pi

```bash
# Build & deploy (auto-detects Pi architecture)
scripts/deploy.sh --host 192.168.4.55 --user pi --release

# Build & deploy, then run on Pi
scripts/run_pi.sh --release --deploy
```

Requires: `sshpass` (optional), `cargo-zigbuild` + `zig` for cross-compilation.

### SVG overlay argument

A positional SVG path is forwarded to the hardware renderer as an overlay:

```bash
cargo run -- /path/to/overlay.svg
```

### Calendar ICS URL

Get your private ICS URL from:
1. Google Calendar → Settings → [your calendar] → **Integrate calendar**
2. Copy the **"Secret address in iCal format"**

No OAuth or API key needed — the secret URL is itself the auth token.

## Architecture

### Data flow

```
Open-Meteo API ─┐
Config TOML ────┤
.env ───────────┤
                 ▼
            api/weather.rs  ──►  model/weather.rs  ──►  panel/weather.rs  ──►  templates/*.svg.j2  ──►  render/template.rs  ──►  render/rasterize.rs
            api/gcalendar.rs ──►  model/calendar.rs ──►  panel/calendar.rs ──┘
                                                                                                         │
                                                                                                         ▼
                                                                                                   SVG bytes
                                                                                                         │
                                                                                         ┌───────────────┴───────────────┐
                                                                                         ▼                               ▼
                                                                                    render/pi.rs                   render/rasterize.rs
                                                                                    (SPI → e-ink)                  (PNG preview)
```

### Template system

Each panel has two SVG Jinja2 templates (light + dark). Templates live in `templates/` as `.svg.j2` files. They are embedded at compile time via `include_dir!`.

Icons from `icons/` are injected as inline SVG `<svg>` fragments via `render::icon_markup()` and `render::icon_markup_unsized()`.

### Colour pipeline

1. SVG rendered at 2× supersampled resolution via `resvg`
2. 2×2 blocks averaged to target resolution
3. Each pixel mapped to the 7-colour ACeP palette using Bayer-ordered dithering
4. Fed to Waveshare EPD driver (SPI) or saved as a simulated PNG preview

## Adding a new panel

1. Create types in `src/model/` (the domain data)
2. Add an API function in `src/api/` (or reuse existing)
3. Create a panel view model in `src/panel/` with `fn render_svg(&self, theme)`
4. Add SVG templates in `templates/` as `my_panel.svg.j2` and `my_panel_dark.svg.j2`
5. Register the panel in `main.rs` by matching on the panel name

## Customisation

- **Layout:** edit templates in `templates/`
- **Icons:** add/replace SVGs in `icons/`
- **Theme:** light/dark via `config.toml` or `EINK_WEATHER_THEME`
- **Metrics:** adjust which data is shown in panel view models

## Development

### Testing

```bash
cargo test
```

Tests cover: Bayer dithering matrix, palette colour mapping, and roundtrip verification. Panel and API logic tests are WIP (contributions welcome).

### Preview without hardware

Set `preview_svg_path` and `preview_png_path` in config, or run with mock data to iterate on template design without SPI/GPIO hardware.

## License

MIT

---

*Created by James Graham, 2023–2026.*
