#!/usr/bin/env bash
# Set up the headless HarmonyOS build toolchain on a Linux x64 machine.
# Idempotent: skips download when the toolchain already exists.
set -e

OHOS_ROOT="${OHOS_ROOT:-$HOME/ohos}"
TOOLS_DIR="$OHOS_ROOT/command-line-tools"
MIRROR="${OHOS_CT_MIRROR:-https://hf-mirror.com}"
VERSION="${OHOS_CT_VERSION:-6.0.1.251}"

if [ -x "$TOOLS_DIR/bin/hvigorw" ]; then
    echo "toolchain already present: $TOOLS_DIR"
else
    ZIP="$OHOS_ROOT/dl/commandline-tools-linux-x64-$VERSION.zip"
    mkdir -p "$OHOS_ROOT/dl"
    if [ ! -f "$ZIP" ]; then
        echo "downloading commandline-tools $VERSION (~2GB) ..."
        curl -L --retry 3 -C - -o "$ZIP" \
            "$MIRROR/csukuangfj/harmonyos-commandline-tools/resolve/main/commandline-tools-linux-x64-$VERSION.zip"
    fi
    echo "unpacking ..."
    unzip -q -o "$ZIP" -d "$OHOS_ROOT"
fi

cat <<EOF

toolchain ready: $TOOLS_DIR
add to your shell (or let scripts/build.sh do it automatically):

  export PATH="$TOOLS_DIR/bin:\$PATH"

verify:
  $TOOLS_DIR/bin/hvigorw -v
EOF
