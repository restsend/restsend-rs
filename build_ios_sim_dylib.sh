#!/bin/bash
set -e

echo "Building restsend-dart dynamic library for iOS Simulator (arm64 + x86_64)..."

# Build for iOS Simulator arm64
cargo build -p restsend-dart --release --target aarch64-apple-ios-sim

# Build for iOS Simulator x86_64
cargo build -p restsend-dart --release --target x86_64-apple-ios

echo "Creating universal iOS Simulator dynamic library..."

LIB_NAME="librestsend_dart.dylib"
IOS_SIM_ARM64="target/aarch64-apple-ios-sim/release/$LIB_NAME"
IOS_SIM_X86="target/x86_64-apple-ios/release/$LIB_NAME"
IOS_SIM_UNIVERSAL="target/ios-sim-universal/release/$LIB_NAME"

# Create output directory
mkdir -p "target/ios-sim-universal/release"

# Combine arm64 and x86_64 into a universal library
lipo -create "$IOS_SIM_ARM64" "$IOS_SIM_X86" -output "$IOS_SIM_UNIVERSAL"

echo "Copying to iOS plugin directory..."

OUTPUT_DIR="dart/restsend_dart/ios"

# Copy universal dylib
cp "$IOS_SIM_UNIVERSAL" "$OUTPUT_DIR/librestsend_dart.dylib"

echo "✅ iOS Simulator dynamic library created at: $OUTPUT_DIR/librestsend_dart.dylib"
echo ""
echo "Now you can run:"
echo "  cd dart/restsend_dart/example"
echo "  flutter clean"
echo "  flutter pub get"
echo "  cd ios && pod install && cd .."
echo "  flutter run -d <ios-simulator-id>"
