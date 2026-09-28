#!/usr/bin/env bash
# Start a local restsend-backend for the integration tests.
# HOST=0.0.0.0 ./backend.sh   # bind all interfaces so an emulator/device on
#                             # the LAN can reach it (then use the LAN IP in-app)
set -e
cd "$(dirname "$0")"
PORT="${PORT:-18123}"
HOST="${HOST:-127.0.0.1}"
DATA_DIR="${DATA_DIR:-/tmp/opencode/restsend-test}"
mkdir -p "$DATA_DIR"
cd "$DATA_DIR"
exec env \
  ADDR="$HOST:$PORT" \
  DATABASE_URL="sqlite://restsend-test.db?mode=rwc" \
  RUN_MIGRATIONS=true \
  API_PREFIX=/api \
  RS_PRESENCE_BACKEND=memory \
  LOG_FILE=logs/restsend-backend.log \
  /home/pi/workspace/rs/restsend-rs/target/debug/restsend-backend
