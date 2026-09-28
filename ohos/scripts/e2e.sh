#!/usr/bin/env bash
# Run the ArkTS SDK e2e tests against a real restsend-backend.
# Reuses an already-running backend on $PORT when healthy; otherwise
# builds+starts one and tears it down on exit.
set -e
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
PORT="${PORT:-18123}"
HEALTH="http://127.0.0.1:$PORT/api/health"

echo "== build backend (cargo) =="
(cd "$ROOT" && cargo build -p restsend-backend)

BACKEND_PID=""
if curl -s --max-time 3 "$HEALTH" | grep -q ok; then
    echo "== backend already running on :$PORT, reusing =="
else
    echo "== start backend on :$PORT =="
    DATA_DIR="${DATA_DIR:-/tmp/opencode/restsend-test}"
    mkdir -p "$DATA_DIR"
    (cd "$DATA_DIR" && ADDR="127.0.0.1:$PORT" \
        DATABASE_URL="sqlite://restsend-test.db?mode=rwc" \
        RUN_MIGRATIONS=true API_PREFIX=/api RS_PRESENCE_BACKEND=memory \
        LOG_FILE=logs/restsend-backend.log \
        nohup "$ROOT/target/debug/restsend-backend" > backend-stdout.log 2>&1 &)
    BACKEND_PID=$!
    for i in $(seq 1 30); do
        curl -s --max-time 2 "$HEALTH" | grep -q ok && break
        sleep 1
    done
    curl -s --max-time 3 "$HEALTH" | grep -q ok || { echo "backend failed to start" >&2; exit 1; }
fi
trap '[ -n "$BACKEND_PID" ] && pkill -P "$BACKEND_PID" 2>/dev/null || true' EXIT

echo "== run e2e tests =="
cd "$ROOT/ohos/harness"
[ -d node_modules ] || npm install --no-audit --no-fund
node build.mjs
node --test test.mjs
