#!/usr/bin/env bash
#
# Runs the Paho MQTT 3.1.1 compliance suite against a broker started by setup.sh.
#
#   ./tests/compliance/run.sh                               run every test
#   ./tests/compliance/run.sh Test.test_retained_messages   run one test
#
# Extra arguments go to unittest (test names, -v, -k PATTERN, ...).
#
# Environment:
#   TELEMQ_HOST  broker host (default localhost)
#   TELEMQ_PORT  broker TCP port (default 1883)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SUITE_DIR="$SCRIPT_DIR/paho.mqtt.testing/interoperability"

[[ -f "$SUITE_DIR/client_test.py" ]] || {
    echo "error: Paho suite not found; run $SCRIPT_DIR/setup.sh first" >&2
    exit 1
}

cd "$SUITE_DIR"

# client_test.py parses its own options with getopt and then calls
# unittest.main(), which re-parses sys.argv and rejects them. Give the suite
# its options via sys.argv and hand unittest only the remaining arguments.
exec python3 -c '
import os, runpy, sys, unittest

unittest_args = sys.argv[1:]
main = unittest.main
unittest.main = lambda *a, **kw: main(*a, argv=["client_test.py"] + unittest_args, **kw)

sys.argv = [
    "client_test.py",
    "--hostname=" + os.environ.get("TELEMQ_HOST", "localhost"),
    "--port=" + os.environ.get("TELEMQ_PORT", "1883"),
]
runpy.run_path("client_test.py", run_name="__main__")
' "$@"
