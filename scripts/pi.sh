#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./common.sh
source "${SCRIPT_DIR}/common.sh"

BUILD_PROFILE="release"
REMOTE_DIR="${REMOTE_DIR:-}"
REMOTE_DIR_SET="0"
if [[ -n "$REMOTE_DIR" ]]; then
	REMOTE_DIR_SET="1"
fi

DEPLOY_EXTRA_ARGS=()
RUN_EXTRA_ARGS=()
REMOTE_ARGS=()
PRINT_ONLY="0"

usage() {
	cat <<'EOF'
Usage: scripts/pi.sh [options] [-- remote_program_args...]

Build for Raspberry Pi on macOS, copy the binary over SSH, and run it remotely.

Options:
	--release             Build/use the release binary (default)
	--host <ip_or_host>   Raspberry Pi host (default: 192.168.4.55)
	--user <username>     SSH username (default: james)
	--port <port>         SSH port (default: 22)
	--password <password> SSH password (optional; use PI_PASSWORD env var)
	--remote-dir <path>   Remote app directory
	--target <triple>     Rust target triple for Pi
	--cross               Build using cross
	--no-cross            Build natively with cargo on the Mac
	--zigbuild            Build using cargo-zigbuild (default)
	--skip-build          Reuse the existing local binary during deploy
	--force               Suppress Mach-O warning during deploy
	--sudo                Run the binary with sudo on the Pi
	--background          Start with nohup and return immediately
	--skip-checks         Skip remote binary/device preflight checks
	--print-only          Print the delegated deploy/run commands without executing them
	-h, --help            Show this help

Arguments after '--' are passed to the remote program.

Examples:
	scripts/pi.sh
	scripts/pi.sh --release
	scripts/pi.sh --sudo -- --path/to/overlay.svg
	scripts/pi.sh --background --skip-build
EOF
}

while [[ $# -gt 0 ]]; do
	case "$1" in
		--release)
			BUILD_PROFILE="release"
			shift
			;;
		--host)
			PI_HOST="$2"
			shift 2
			;;
		--user)
			PI_USER="$2"
			shift 2
			;;
		--port)
			PI_PORT="$2"
			shift 2
			;;
		--password)
			PI_PASSWORD="$2"
			shift 2
			;;
		--remote-dir)
			REMOTE_DIR="$2"
			REMOTE_DIR_SET="1"
			shift 2
			;;
		--target|--cross|--no-cross|--zigbuild|--skip-build|--force)
			DEPLOY_EXTRA_ARGS+=("$1")
			if [[ "$1" == "--target" ]]; then
				DEPLOY_EXTRA_ARGS+=("$2")
				shift 2
			else
				shift
			fi
			;;
		--sudo|--background|--skip-checks)
			RUN_EXTRA_ARGS+=("$1")
			shift
			;;
		--print-only)
			PRINT_ONLY="1"
			shift
			;;
		-h|--help)
			usage
			exit 0
			;;
		--)
			shift
			REMOTE_ARGS=("$@")
			break
			;;
		*)
			echo "Unknown option: $1" >&2
			usage
			exit 1
			;;
	esac
done

if [[ "$REMOTE_DIR_SET" != "1" ]]; then
	REMOTE_DIR="/home/${PI_USER}/apps/${APP_NAME}"
fi

COMMON_ARGS=(--host "$PI_HOST" --user "$PI_USER" --port "$PI_PORT" --remote-dir "$REMOTE_DIR")
if [[ "$BUILD_PROFILE" == "release" ]]; then
	COMMON_ARGS+=(--release)
fi
if [[ -n "$PI_PASSWORD" ]]; then
	COMMON_ARGS+=(--password "$PI_PASSWORD")
fi

DEPLOY_CMD=("${SCRIPT_DIR}/deploy.sh" "${COMMON_ARGS[@]}")
RUN_CMD=("${SCRIPT_DIR}/run_pi.sh" "${COMMON_ARGS[@]}")
if [[ ${#DEPLOY_EXTRA_ARGS[@]} -gt 0 ]]; then
	DEPLOY_CMD+=("${DEPLOY_EXTRA_ARGS[@]}")
fi
if [[ ${#RUN_EXTRA_ARGS[@]} -gt 0 ]]; then
	RUN_CMD+=("${RUN_EXTRA_ARGS[@]}")
fi
if [[ ${#REMOTE_ARGS[@]} -gt 0 ]]; then
	RUN_CMD+=(-- "${REMOTE_ARGS[@]}")
fi

if [[ "$PRINT_ONLY" == "1" ]]; then
	echo "Deploy command: $(join_shell_words "${DEPLOY_CMD[@]}")"
	echo "Run command: $(join_shell_words "${RUN_CMD[@]}")"
	exit 0
fi

"${DEPLOY_CMD[@]}"
"${RUN_CMD[@]}"