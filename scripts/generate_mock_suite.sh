#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="${1:-$ROOT_DIR/tmp/mock-suite}"
THEME="${EINK_WEATHER_THEME:-light}"

mkdir -p "$OUT_DIR"

PRESETS=(
  clear
  partly-cloudy
  cloudy
  fog
  drizzle
  rain
  showers
  snow
  snow-showers
  thunder
  hail-thunder
)

echo "Generating mock forecast suite in: $OUT_DIR"
echo "Theme: $THEME"

i=1
for preset in "${PRESETS[@]}"; do
  seq="$(printf "%02d" "$i")"
  svg_path="$OUT_DIR/${seq}-${preset}.svg"
  png_path="$OUT_DIR/${seq}-${preset}.png"

  echo "[$seq/${#PRESETS[@]}] Rendering preset: $preset"

  (
    cd "$ROOT_DIR"
    EINK_WEATHER_FORECAST_SOURCE=mock \
    EINK_WEATHER_MOCK_PRESET="$preset" \
    EINK_WEATHER_MOCK_NIGHT=0 \
    EINK_WEATHER_HOURS=6 \
    EINK_WEATHER_THEME="$THEME" \
    EINK_WEATHER_REFRESH_SECS=0 \
    EINK_WEATHER_PREVIEW_SVG_PATH="$svg_path" \
    EINK_WEATHER_PREVIEW_PNG_PATH="$png_path" \
    cargo run >/dev/null
  )

  i=$((i + 1))
done

echo "Done. Generated $((${#PRESETS[@]} * 2)) files in $OUT_DIR"
