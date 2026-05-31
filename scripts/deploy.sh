#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
# shellcheck source=./common.sh
source "${SCRIPT_DIR}/common.sh"

BUILD_PROFILE="debug"
REMOTE_DIR="${REMOTE_DIR:-}"
REMOTE_DIR_SET="0"
if [[ -n "$REMOTE_DIR" ]]; then
	REMOTE_DIR_SET="1"
fi
FORCE_DEPLOY="0"
SKIP_BUILD="0"
USE_CROSS="${USE_CROSS:-2}"
PI_TARGET="${PI_TARGET:-}"

usage() {
	cat <<'EOF'
Usage: scripts/deploy.sh [options]

Builds (optional) and copies the binary to a Raspberry Pi over SSH/SCP.

Options:
	--release             Build/use release binary (target/release/eink-weather)
	--skip-build          Do not run cargo build before copying
	--host <ip_or_host>   Raspberry Pi host (default: 192.168.4.55)
	--user <username>     SSH username (default: james)
	--port <port>         SSH port (default: 22)
	--password <password> SSH password (optional; use PI_PASSWORD env var)
	--target <triple>     Rust target triple for Pi (auto-detected by default)
	--cross               Build for Linux target using cross
	--no-cross            Build natively with cargo (macOS binary)
	--zigbuild            Build for Linux target using cargo-zigbuild (default)
	--remote-dir <path>   Destination directory on Pi
	--force               Suppress Mach-O warning when copying macOS binary
	-h, --help            Show this help

Environment overrides:
	PI_HOST, PI_USER, PI_PORT, PI_PASSWORD, PI_TARGET, USE_CROSS, REMOTE_DIR

Examples:
	scripts/deploy.sh --release
	scripts/deploy.sh --target aarch64-unknown-linux-gnu --release
	PI_PASSWORD='my-pass' scripts/deploy.sh --host 192.168.4.55 --user james
EOF
}

while [[ $# -gt 0 ]]; do
	case "$1" in
		--release)
			BUILD_PROFILE="release"
			shift
			;;
		--skip-build)
			SKIP_BUILD="1"
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
		--target)
			PI_TARGET="$2"
			shift 2
			;;
		--cross)
			USE_CROSS="1"
			shift
			;;
		--no-cross)
			USE_CROSS="0"
			shift
			;;
		--zigbuild)
			USE_CROSS="2"
			shift
			;;
		--remote-dir)
			REMOTE_DIR="$2"
			REMOTE_DIR_SET="1"
			shift 2
			;;
		--force)
			FORCE_DEPLOY="1"
			shift
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

if [[ "$REMOTE_DIR_SET" != "1" ]]; then
	REMOTE_DIR="/home/${PI_USER}/apps/${APP_NAME}"
fi

REMOTE_TMP_BIN="${REMOTE_DIR}/${APP_NAME}.new"
REMOTE_FINAL_BIN="${REMOTE_DIR}/${APP_NAME}"

init_ssh_prefix

if [[ "$USE_CROSS" != "0" ]] && [[ -z "$PI_TARGET" ]]; then
	echo "Detecting Raspberry Pi architecture on ${PI_USER}@${PI_HOST}..."
	REMOTE_ARCH="$(run_ssh -p "$PI_PORT" -o StrictHostKeyChecking=accept-new "${PI_USER}@${PI_HOST}" "uname -m" | tr -d '[:space:]')"
	if ! PI_TARGET="$(target_from_arch "$REMOTE_ARCH")"; then
		echo "Could not map remote architecture '${REMOTE_ARCH}' to a Rust target." >&2
		echo "Pass an explicit target, for example: --target aarch64-unknown-linux-gnu" >&2
		exit 1
	fi
	echo "Detected architecture ${REMOTE_ARCH} -> target ${PI_TARGET}"
fi

if [[ "$USE_CROSS" != "0" ]] && [[ -z "$PI_TARGET" ]]; then
	echo "PI target is required when building for Linux (use --target or ensure Pi is reachable for auto-detect)." >&2
	exit 1
fi

if [[ "$USE_CROSS" != "0" ]]; then
	LOCAL_BIN="${REPO_ROOT}/target/${PI_TARGET}/${BUILD_PROFILE}/${APP_NAME}"
else
	LOCAL_BIN="${REPO_ROOT}/target/${BUILD_PROFILE}/${APP_NAME}"
fi

if [[ "$SKIP_BUILD" != "1" ]]; then
	if [[ "$USE_CROSS" == "1" ]]; then
		BUILD_ARGS=(--builder cross --target "$PI_TARGET")
		if [[ "$BUILD_PROFILE" == "release" ]]; then
			BUILD_ARGS+=(--release)
		fi
		"${SCRIPT_DIR}/build_pi.sh" "${BUILD_ARGS[@]}"
	elif [[ "$USE_CROSS" == "2" ]]; then
		BUILD_ARGS=(--builder zigbuild --target "$PI_TARGET")
		if [[ "$BUILD_PROFILE" == "release" ]]; then
			BUILD_ARGS+=(--release)
		fi
		"${SCRIPT_DIR}/build_pi.sh" "${BUILD_ARGS[@]}"
	else
		cd "$REPO_ROOT"
		echo "Building ${APP_NAME} (${BUILD_PROFILE}) natively with cargo..."
		if [[ "$BUILD_PROFILE" == "release" ]]; then
			cargo build --release
		else
			cargo build
		fi
	fi
fi

if [[ ! -f "$LOCAL_BIN" ]]; then
	echo "Binary not found at ${LOCAL_BIN}" >&2
	echo "Try running without --skip-build or with --release if needed." >&2
	exit 1
fi

LOCAL_FILE_INFO="$(file "$LOCAL_BIN")"
if [[ "$LOCAL_FILE_INFO" == *"Mach-O"* ]] && [[ "$FORCE_DEPLOY" != "1" ]]; then
	cat <<EOF
Warning: copying a macOS Mach-O binary:
	${LOCAL_FILE_INFO}

Raspberry Pi runs Linux, so this binary usually will not execute there.
Copying anyway because this script supports transfer-only workflows.
Use --force to suppress this warning.
EOF
fi

echo "Preparing destination on ${PI_USER}@${PI_HOST}:${REMOTE_DIR}..."

run_ssh -p "$PI_PORT" -o StrictHostKeyChecking=accept-new \
	"${PI_USER}@${PI_HOST}" "mkdir -p '$REMOTE_DIR'"

echo "Copying ${LOCAL_BIN} -> ${PI_HOST}:${REMOTE_TMP_BIN}..."
run_scp -P "$PI_PORT" "$LOCAL_BIN" "${PI_USER}@${PI_HOST}:${REMOTE_TMP_BIN}"

echo "Finalizing remote binary permissions..."
run_ssh -p "$PI_PORT" "${PI_USER}@${PI_HOST}" \
	"chmod +x '$REMOTE_TMP_BIN' && mv '$REMOTE_TMP_BIN' '$REMOTE_FINAL_BIN'"

echo "Done. Deployed to ${PI_USER}@${PI_HOST}:${REMOTE_FINAL_BIN}"
