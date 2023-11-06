#!/bin/sh
cargo build --target aarch64-apple-ios-sim --target aarch64-apple-ios --target x86_64-apple-darwin
./scripts/xcframework.sh --debug --dev
