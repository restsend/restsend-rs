#!/bin/bash
set -e

echo "Building restsend-dart for macOS (arm64)..."

# Build for macOS arm64 only (Apple Silicon)
cargo build -p restsend-dart --release --target aarch64-apple-darwin

echo "Copying library for Flutter macOS..."

LIB_DIR="target/aarch64-apple-darwin/release"
DYLIB_NAME="librestsend_dart.dylib"
STATIC_NAME="librestsend_dart.a"

OUTPUT_DIR="dart/restsend_dart/macos"
mkdir -p "$OUTPUT_DIR"

# Copy the dynamic library
if [ -f "$LIB_DIR/$DYLIB_NAME" ]; then
    cp "$LIB_DIR/$DYLIB_NAME" "$OUTPUT_DIR/$DYLIB_NAME"
    echo "✅ Dynamic library copied to: $OUTPUT_DIR/$DYLIB_NAME"
fi

# Copy the static library
if [ -f "$LIB_DIR/$STATIC_NAME" ]; then
    cp "$LIB_DIR/$STATIC_NAME" "$OUTPUT_DIR/$STATIC_NAME"
    echo "✅ Static library copied to: $OUTPUT_DIR/$STATIC_NAME"
fi

echo ""
echo "Note: This build is for Apple Silicon (arm64) only."
echo "Without Xcode, xcframework cannot be created, but the dylib should work."
echo ""
echo "Now you can run:"
echo "  cd dart/restsend_dart/example"
echo "  flutter clean"
echo "  flutter pub get"
echo "  flutter run -d macos"
