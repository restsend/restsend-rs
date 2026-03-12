#!/bin/bash
set -e

echo "Building restsend-dart for iOS device (arm64) + Simulator (arm64 + x86_64)..."

# ── 1. Build all targets ──────────────────────────────────────────────────────

# iOS device (real device)
cargo build -p restsend-dart --release --target aarch64-apple-ios

# iOS Simulator arm64 (Apple Silicon Mac)
cargo build -p restsend-dart --release --target aarch64-apple-ios-sim

# iOS Simulator x86_64 (Intel Mac)
cargo build -p restsend-dart --release --target x86_64-apple-ios

echo "Creating universal iOS Simulator library..."

LIB_NAME="librestsend_dart.a"
IOS_DEVICE_ARM64="target/aarch64-apple-ios/release/$LIB_NAME"
IOS_SIM_ARM64="target/aarch64-apple-ios-sim/release/$LIB_NAME"
IOS_SIM_X86="target/x86_64-apple-ios/release/$LIB_NAME"
IOS_SIM_UNIVERSAL="target/ios-sim-universal/release/$LIB_NAME"

mkdir -p "target/ios-sim-universal/release"

# Combine arm64-sim + x86_64 into a universal simulator library
lipo -create "$IOS_SIM_ARM64" "$IOS_SIM_X86" -output "$IOS_SIM_UNIVERSAL"

echo "Creating xcframework (device + simulator)..."

OUTPUT_DIR="dart/restsend_dart/ios"
XCFRAMEWORK="$OUTPUT_DIR/restsend_dart_ffi.xcframework"

# Remove existing xcframework
if [ -d "$XCFRAMEWORK" ]; then
    rm -rf "$XCFRAMEWORK"
fi

# Create xcframework with both device and simulator slices
xcodebuild -create-xcframework \
    -library "$IOS_DEVICE_ARM64" \
    -library "$IOS_SIM_UNIVERSAL" \
    -output "$XCFRAMEWORK"

# Remove simulator-only dylib that causes linker errors on real devices
if [ -f "$OUTPUT_DIR/librestsend_dart.dylib" ]; then
    echo "Removing simulator-only librestsend_dart.dylib..."
    rm -f "$OUTPUT_DIR/librestsend_dart.dylib"
fi

echo "✅ iOS xcframework created at: $XCFRAMEWORK"
echo ""
echo "Contents:"
ls -lh "$XCFRAMEWORK/"
echo ""
echo "Next steps:"
echo "  cd dart/restsend_dart/example"
echo "  flutter clean && flutter pub get"
echo "  cd ios && pod install && cd .."
echo "  flutter run -d <device-id>"
