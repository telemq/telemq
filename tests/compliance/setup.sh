#!/usr/bin/env bash
#
# Prepares the Eclipse Paho MQTT 3.1.1 compliance suite and starts TeleMQ
# with the compliance config from this directory.
#
#   ./tests/compliance/setup.sh          install Paho, build and start TeleMQ
#   ./tests/compliance/setup.sh stop     stop the broker started by this script
#
# Environment:
#   TELEMQ_PORT  TCP port for the broker (default 1883)
#   PAHO_REF     paho.mqtt.testing commit/branch to check out (default: pinned)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

PAHO_URL="https://github.com/eclipse/paho.mqtt.testing.git"
PAHO_REF="${PAHO_REF:-9d7bb80bb8b9d9cfc0b52f8cb4c1916401281103}"
PAHO_DIR="$SCRIPT_DIR/paho.mqtt.testing"

TELEMQ_PORT="${TELEMQ_PORT:-1883}"
TELEMQ_BIN="$REPO_ROOT/target/release/telemq"
CONFIG_FILE="$SCRIPT_DIR/telemq.toml"
PID_FILE="$SCRIPT_DIR/broker.pid"
LOG_FILE="$SCRIPT_DIR/broker.log"

log() { printf '==> %s\n' "$*"; }
die() { printf 'error: %s\n' "$*" >&2; exit 1; }

require() {
    for cmd in "$@"; do
        command -v "$cmd" >/dev/null 2>&1 || die "'$cmd' is required but not installed"
    done
}

port_open() {
    (exec 3<>"/dev/tcp/127.0.0.1/$TELEMQ_PORT") 2>/dev/null
}

stop_broker() {
    if [[ -f "$PID_FILE" ]]; then
        local pid
        pid="$(cat "$PID_FILE")"
        if kill -0 "$pid" 2>/dev/null; then
            log "Stopping TeleMQ (pid $pid)"
            kill "$pid"
            for _ in $(seq 1 50); do
                kill -0 "$pid" 2>/dev/null || break
                sleep 0.1
            done
        fi
        rm -f "$PID_FILE"
    fi
}

install_paho() {
    if [[ ! -d "$PAHO_DIR/.git" ]]; then
        log "Cloning paho.mqtt.testing into $PAHO_DIR"
        git clone --quiet "$PAHO_URL" "$PAHO_DIR"
    fi
    if ! git -C "$PAHO_DIR" cat-file -e "$PAHO_REF^{commit}" 2>/dev/null; then
        git -C "$PAHO_DIR" fetch --quiet origin
    fi
    log "Checking out paho.mqtt.testing at $PAHO_REF"
    git -C "$PAHO_DIR" checkout --quiet "$PAHO_REF"
}

build_broker() {
    log "Building TeleMQ (release)"
    cargo build --release --quiet -p telemq --manifest-path "$REPO_ROOT/Cargo.toml"
}

start_broker() {
    stop_broker
    port_open && die "port $TELEMQ_PORT is already in use; set TELEMQ_PORT or stop whatever is listening"

    log "Starting TeleMQ on port $TELEMQ_PORT (log: $LOG_FILE)"
    # Run from this directory so relative paths in telemq.toml (auth_file) resolve here.
    (cd "$SCRIPT_DIR" && exec nohup "$TELEMQ_BIN" --config="$CONFIG_FILE" "$TELEMQ_PORT" >"$LOG_FILE" 2>&1) &
    echo $! >"$PID_FILE"

    for _ in $(seq 1 100); do
        port_open && return 0
        kill -0 "$(cat "$PID_FILE")" 2>/dev/null || break
        sleep 0.1
    done
    tail -n 20 "$LOG_FILE" >&2 || true
    stop_broker
    die "TeleMQ did not start listening on port $TELEMQ_PORT"
}

case "${1:-}" in
    stop)
        stop_broker
        exit 0
        ;;
    "") ;;
    *)
        die "unknown argument '$1' (usage: $0 [stop])"
        ;;
esac

require git python3 cargo
install_paho
build_broker
start_broker

cat <<EOF

TeleMQ is running (pid $(cat "$PID_FILE")) on port $TELEMQ_PORT.

Run the compliance suite:
    TELEMQ_PORT=$TELEMQ_PORT $SCRIPT_DIR/run.sh

Stop the broker:
    $0 stop
EOF
