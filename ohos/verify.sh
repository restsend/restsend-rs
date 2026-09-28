#!/usr/bin/env bash
# One-shot Linux verification: toolchain -> build -> e2e tests.
set -e
ROOT="$(cd "$(dirname "$0")" && pwd)"

echo "========== 1/3 toolchain =========="
"$ROOT/scripts/setup-linux-env.sh"

echo "========== 2/3 build (har + hap) =========="
"$ROOT/scripts/build.sh"

echo "========== 3/3 e2e tests =========="
"$ROOT/scripts/e2e.sh"

echo ""
echo "ALL CHECKS PASSED"
