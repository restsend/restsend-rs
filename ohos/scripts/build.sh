#!/usr/bin/env bash
# Build the SDK HAR and the demo app HAP on Linux (no DevEco Studio needed).
set -e
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OHOS_ROOT="${OHOS_ROOT:-$HOME/ohos}"
TOOLS_DIR="$OHOS_ROOT/command-line-tools"

if [ ! -x "$TOOLS_DIR/bin/hvigorw" ]; then
    echo "toolchain missing; run ohos/scripts/setup-linux-env.sh first" >&2
    exit 1
fi
export PATH="$TOOLS_DIR/bin:$PATH"

cd "$ROOT/ohos"
ohpm install
echo "== assembleHar (sdk) =="
hvigorw --mode module -p module=restsend@default assembleHar --no-daemon
echo "== assembleHap (app) =="
hvigorw assembleHap --no-daemon

ls -lh Library/restsend/build/default/outputs/default/restsend.har
ls -lh entry/build/default/outputs/default/entry-default-unsigned.hap
