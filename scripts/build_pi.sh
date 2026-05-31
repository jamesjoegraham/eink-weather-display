#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

APP_NAME="eink-weather"
TARGET="${PI_TARGET:-aarch64-unknown-linux-gnu}"
PROFILE="debug"
BUILDER="${PI_BUILDER:-zigbuild}" # zigbuild | cross | cargo

usage() {
	cat <<'EOF'
Usage: scripts/build_pi.sh [options]

Build Linux binaries for Raspberry Pi from macOS.

Options:
	--release            Build release binary
	--target <triple>    Rust target triple (default: aarch64-unknown-linux-gnu)
	--builder <name>     zigbuild (default), cross, or cargo
	-h, --help           Show help

Environment overrides:
	PI_TARGET, PI_BUILDER

Examples:
	scripts/build_pi.sh
	scripts/build_pi.sh --release
	scripts/build_pi.sh --target armv7-unknown-linux-gnueabihf
	scripts/build_pi.sh --builder cross --release
EOF
}

while [[ $# -gt 0 ]]; do
	case "$1" in
		--release)
			PROFILE="release"
			shift
			;;
		--target)
			TARGET="$2"
			shift 2
			;;
		--builder)
			BUILDER="$2"
			shift 2
			;;
		-h|--help)
			usage
			exit 0
			;;
		*)
			echo "Unknown option: $1" >&2
			usage
			exit 1
			;;
	esac
done

ensure_target() {
	cd "$REPO_ROOT"
	echo "Ensuring Rust target ${TARGET} is installed..."
	rustup target add "$TARGET"
}

build_with_zigbuild() {
	cd "$REPO_ROOT"
	command -v cargo-zigbuild >/dev/null 2>&1 || {
		echo "cargo-zigbuild is required. Install with: cargo install cargo-zigbuild" >&2
		exit 1
	}
	command -v zig >/dev/null 2>&1 || {
		echo "zig is required. Install with: brew install zig" >&2
		exit 1
	}

	echo "Building with cargo-zigbuild for ${TARGET} (${PROFILE})..."
	if [[ "$PROFILE" == "release" ]]; then
		cargo zigbuild --release --target "$TARGET"
	else
		cargo zigbuild --target "$TARGET"
	fi
}

build_with_cross() {
	cd "$REPO_ROOT"
	command -v cross >/dev/null 2>&1 || {
		echo "cross is required. Install with: cargo install cross --git https://github.com/cross-rs/cross" >&2
		exit 1
	}

	ensure_target
	echo "Building with cross for ${TARGET} (${PROFILE})..."
	if [[ "$PROFILE" == "release" ]]; then
		CROSS_CUSTOM_TOOLCHAIN=1 cross build --release --target "$TARGET"
	else
		CROSS_CUSTOM_TOOLCHAIN=1 cross build --target "$TARGET"
	fi
}

build_with_cargo() {
	cd "$REPO_ROOT"
	ensure_target
	echo "Building with cargo for ${TARGET} (${PROFILE})..."
	if [[ "$PROFILE" == "release" ]]; then
		cargo build --release --target "$TARGET"
	else
		cargo build --target "$TARGET"
	fi
}

case "$BUILDER" in
	zigbuild)
		build_with_zigbuild
		;;
	cross)
		build_with_cross
		;;
	cargo)
		build_with_cargo
		;;
	*)
		echo "Invalid builder '${BUILDER}'. Use: zigbuild, cross, or cargo." >&2
		exit 1
		;;
esac

BIN_PATH="${REPO_ROOT}/target/${TARGET}/${PROFILE}/${APP_NAME}"
if [[ ! -f "$BIN_PATH" ]]; then
	echo "Build completed but binary not found at ${BIN_PATH}" >&2
	exit 1
fi

echo "Built: ${BIN_PATH}"
file "$BIN_PATH"
