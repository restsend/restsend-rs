#!/bin/bash
set -e

echo "Building restsend-dart for iOS Simulator (arm64)..."

# Build for iOS Simulator arm64
cargo build -p restsend-dart --release --target aarch64-apple-ios-sim

echo "Creating xcframework for iOS Simulator..."

LIB_NAME="librestsend_dart.a"
IOS_SIM_LIB="target/aarch64-apple-ios-sim/release/$LIB_NAME"
OUTPUT_DIR="dart/restsend_dart/ios"
XCFRAMEWORK="$OUTPUT_DIR/restsend_dart_ffi.xcframework"

# Remove existing xcframework if present
if [ -d "$XCFRAMEWORK" ]; then
    rm -rf "$XCFRAMEWORK"
fi

# Create xcframework with just iOS Simulator support
xcodebuild -create-xcframework \
    -library "$IOS_SIM_LIB" \
    -output "$XCFRAMEWORK"

echo "✅ iOS Simulator framework created at: $XCFRAMEWORK"
echo ""
echo "Now you can run:"
echo "  cd dart/restsend_dart/example"
echo "  flutter clean"
echo "  flutter pub get"
echo "  cd ios && pod install && cd .."
echo "  flutter run -d <ios-simulator-id>"
