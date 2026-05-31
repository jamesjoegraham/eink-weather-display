#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./common.sh
source "${SCRIPT_DIR}/common.sh"

BUILD_PROFILE="debug"
REMOTE_DIR="${REMOTE_DIR:-}"
REMOTE_DIR_SET="0"
if [[ -n "$REMOTE_DIR" ]]; then
	REMOTE_DIR_SET="1"
fi
USE_SUDO="0"
RUN_IN_BACKGROUND="0"
DEPLOY_FIRST="0"
PRINT_ONLY="0"
SKIP_CHECKS="0"

REMOTE_ARGS=()

usage() {
	cat <<'EOF'
Usage: scripts/run_pi.sh [options] [-- remote_program_args...]

Run the deployed eink-weather binary on a Raspberry Pi over SSH.

Options:
	--release             Use the release remote binary path
	--host <ip_or_host>   Raspberry Pi host (default: 192.168.4.55)
	--user <username>     SSH username (default: james)
	--port <port>         SSH port (default: 22)
	--password <password> SSH password (optional; use PI_PASSWORD env var)
	--remote-dir <path>   Remote app directory
	--sudo                Run the binary with sudo on the Pi
	--background          Start with nohup and return immediately
	--deploy              Run scripts/deploy.sh first using matching connection settings
	--print-only          Print the remote command without executing it
	--skip-checks         Skip remote binary/device preflight checks
	-h, --help            Show this help

Arguments after '--' are passed to the remote program.

Environment overrides:
	PI_HOST, PI_USER, PI_PORT, PI_PASSWORD, REMOTE_DIR

Examples:
	scripts/run_pi.sh
	scripts/run_pi.sh --release --deploy
	scripts/run_pi.sh --sudo -- --path/to/overlay.svg
	scripts/run_pi.sh --background --deploy
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
		--sudo)
			USE_SUDO="1"
			shift
			;;
		--background)
			RUN_IN_BACKGROUND="1"
			shift
			;;
		--deploy)
			DEPLOY_FIRST="1"
			shift
			;;
		--print-only)
			PRINT_ONLY="1"
			shift
			;;
		--skip-checks)
			SKIP_CHECKS="1"
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

init_ssh_prefix

REMOTE_FINAL_BIN="${REMOTE_DIR}/${APP_NAME}"

run_remote_preflight() {
	local checks=()
	checks+=("cd $(shell_quote "$REMOTE_DIR")")
	checks+=("[[ -x $(shell_quote "$REMOTE_FINAL_BIN") ]] || { echo 'Missing executable: $REMOTE_FINAL_BIN' >&2; exit 1; }")
	checks+=("[[ -e /dev/spidev0.0 ]] || { echo 'Missing device: /dev/spidev0.0 (enable SPI on the Pi)' >&2; exit 1; }")
	checks+=("[[ -e /dev/gpiochip0 ]] || { echo 'Missing device: /dev/gpiochip0' >&2; exit 1; }")

	local remote_check_command
	remote_check_command="$(join_shell_words bash -lc "$(printf '%s && %s && %s && %s' "${checks[0]}" "${checks[1]}" "${checks[2]}" "${checks[3]}")")"

	echo "Checking remote executable and hardware devices..."
	run_ssh -p "$PI_PORT" -o StrictHostKeyChecking=accept-new \
		"${PI_USER}@${PI_HOST}" "$remote_check_command"
}

if [[ "$DEPLOY_FIRST" == "1" ]]; then
	DEPLOY_ARGS=(--host "$PI_HOST" --user "$PI_USER" --port "$PI_PORT" --remote-dir "$REMOTE_DIR")
	if [[ "$BUILD_PROFILE" == "release" ]]; then
		DEPLOY_ARGS+=(--release)
	fi
	if [[ -n "$PI_PASSWORD" ]]; then
		DEPLOY_ARGS+=(--password "$PI_PASSWORD")
	fi

	echo "Deploying ${APP_NAME} to ${PI_USER}@${PI_HOST}..."
	"$(dirname "$0")/deploy.sh" "${DEPLOY_ARGS[@]}"
fi

if [[ "$PRINT_ONLY" != "1" ]] && [[ "$SKIP_CHECKS" != "1" ]]; then
	run_remote_preflight
fi

PROGRAM_CMD=()
if [[ "$USE_SUDO" == "1" ]]; then
	PROGRAM_CMD+=(sudo)
fi
PROGRAM_CMD+=("$REMOTE_FINAL_BIN")
if [[ ${#REMOTE_ARGS[@]} -gt 0 ]]; then
	PROGRAM_CMD+=("${REMOTE_ARGS[@]}")
fi

PROGRAM_CMD_STR="$(join_shell_words "${PROGRAM_CMD[@]}")"
REMOTE_DIR_QUOTED="$(shell_quote "$REMOTE_DIR")"

if [[ "$RUN_IN_BACKGROUND" == "1" ]]; then
	REMOTE_COMMAND="cd ${REMOTE_DIR_QUOTED} && nohup ${PROGRAM_CMD_STR} > ${APP_NAME}.log 2>&1 < /dev/null & echo Started ${APP_NAME} with PID \$!"
else
	REMOTE_COMMAND="cd ${REMOTE_DIR_QUOTED} && exec ${PROGRAM_CMD_STR}"
fi

echo "Remote command: ${REMOTE_COMMAND}"

if [[ "$PRINT_ONLY" == "1" ]]; then
	exit 0
fi

run_ssh -t -p "$PI_PORT" -o StrictHostKeyChecking=accept-new \
	"${PI_USER}@${PI_HOST}" "$REMOTE_COMMAND"
