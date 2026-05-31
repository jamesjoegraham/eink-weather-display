#!/usr/bin/env bash

if [[ -n "${_EINK_WEATHER_PI_COMMON_SH:-}" ]]; then
	return 0 2>/dev/null || exit 0
fi

_EINK_WEATHER_PI_COMMON_SH=1

PI_HOST="${PI_HOST:-192.168.4.55}"
PI_USER="${PI_USER:-james}"
PI_PORT="${PI_PORT:-22}"
PI_PASSWORD="${PI_PASSWORD:-}"
APP_NAME="${APP_NAME:-eink-weather}"

SSH_PREFIX=()

shell_quote() {
	printf "%q" "$1"
}

join_shell_words() {
	local joined=""
	local arg
	for arg in "$@"; do
		if [[ -n "$joined" ]]; then
			joined+=" "
		fi
		joined+="$(shell_quote "$arg")"
	done
	printf '%s' "$joined"
}

target_from_arch() {
	case "$1" in
		aarch64|arm64)
			echo "aarch64-unknown-linux-gnu"
			;;
		armv7l|armv7*)
			echo "armv7-unknown-linux-gnueabihf"
			;;
		armv6l)
			echo "arm-unknown-linux-gnueabihf"
			;;
		x86_64|amd64)
			echo "x86_64-unknown-linux-gnu"
			;;
		*)
			return 1
			;;
	esac
}

init_ssh_prefix() {
	if [[ -n "$PI_PASSWORD" ]] && command -v sshpass >/dev/null 2>&1; then
		SSH_PREFIX=(sshpass -p "$PI_PASSWORD")
	elif [[ -n "$PI_PASSWORD" ]]; then
		SSH_PREFIX=()
		echo "PI_PASSWORD is set but sshpass is not installed; SSH/SCP will prompt interactively."
	else
		SSH_PREFIX=()
	fi
}

run_ssh() {
	if [[ ${#SSH_PREFIX[@]} -gt 0 ]]; then
		"${SSH_PREFIX[@]}" ssh "$@"
	else
		ssh "$@"
	fi
}

run_scp() {
	if [[ ${#SSH_PREFIX[@]} -gt 0 ]]; then
		"${SSH_PREFIX[@]}" scp "$@"
	else
		scp "$@"
	fi
}