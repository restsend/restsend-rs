#!/usr/bin/env bash
# Build the SDK bundle and run the integration tests against a local backend.
set -e
cd "$(dirname "$0")"
node build.mjs
node --test test.mjs
