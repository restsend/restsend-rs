#!/bin/bash
set -e

echo "Building restsend-dart for iOS device (arm64) + Simulator (arm64 + x86_64)..."

FRAMEWORK_NAME="restsend_dart_ffi"
DYLIB_NAME="librestsend_dart.dylib"
OUTPUT_DIR="dart/restsend_dart/ios"

# ── 1. Build all targets ──────────────────────────────────────────────────────

# iOS device (real device)
cargo build -p restsend-dart --release --target aarch64-apple-ios

# iOS Simulator arm64 (Apple Silicon Mac)
cargo build -p restsend-dart --release --target aarch64-apple-ios-sim

# iOS Simulator x86_64 (Intel Mac)
cargo build -p restsend-dart --release --target x86_64-apple-ios

# ── 2. Fix install names for all dylibs ───────────────────────────────────────

echo "Setting install names..."

# Fix install name so iOS can locate the framework at runtime via @rpath
install_name_tool -id "@rpath/$FRAMEWORK_NAME.framework/$FRAMEWORK_NAME" "target/aarch64-apple-ios/release/$DYLIB_NAME"
install_name_tool -id "@rpath/$FRAMEWORK_NAME.framework/$FRAMEWORK_NAME" "target/aarch64-apple-ios-sim/release/$DYLIB_NAME"
install_name_tool -id "@rpath/$FRAMEWORK_NAME.framework/$FRAMEWORK_NAME" "target/x86_64-apple-ios/release/$DYLIB_NAME"

# ── 3. Create device framework ───────────────────────────────────────────────

echo "Creating device framework..."

DEVICE_FRAMEWORK_DIR="target/ios-frameworks/device/$FRAMEWORK_NAME.framework"
mkdir -p "$DEVICE_FRAMEWORK_DIR/Headers"

# Copy device dylib as framework executable
cp "target/aarch64-apple-ios/release/$DYLIB_NAME" "$DEVICE_FRAMEWORK_DIR/$FRAMEWORK_NAME"

# Create Info.plist for device framework
cat > "$DEVICE_FRAMEWORK_DIR/Info.plist" << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleIdentifier</key>
    <string>org.restsend.dart-ffi</string>
    <key>CFBundleName</key>
    <string>restsend_dart_ffi</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>CFBundlePackageType</key>
    <string>FMWK</string>
    <key>CFBundleExecutable</key>
    <string>restsend_dart_ffi</string>
    <key>MinimumOSVersion</key>
    <string>11.0</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
</dict>
</plist>
EOF

# ── 4. Create universal simulator framework ───────────────────────────────────

echo "Creating universal simulator framework..."

# Combine simulator arm64 + x86_64 into universal dylib
mkdir -p "target/ios-sim-universal/release"
lipo -create \
    "target/aarch64-apple-ios-sim/release/$DYLIB_NAME" \
    "target/x86_64-apple-ios/release/$DYLIB_NAME" \
    -output "target/ios-sim-universal/release/$DYLIB_NAME"

SIM_FRAMEWORK_DIR="target/ios-frameworks/simulator/$FRAMEWORK_NAME.framework"
mkdir -p "$SIM_FRAMEWORK_DIR/Headers"

# Copy universal simulator dylib as framework executable
cp "target/ios-sim-universal/release/$DYLIB_NAME" "$SIM_FRAMEWORK_DIR/$FRAMEWORK_NAME"

# Create Info.plist for simulator framework
cat > "$SIM_FRAMEWORK_DIR/Info.plist" << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleIdentifier</key>
    <string>org.restsend.dart-ffi</string>
    <key>CFBundleName</key>
    <string>restsend_dart_ffi</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>CFBundlePackageType</key>
    <string>FMWK</string>
    <key>CFBundleExecutable</key>
    <string>restsend_dart_ffi</string>
    <key>MinimumOSVersion</key>
    <string>11.0</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
</dict>
</plist>
EOF

# ── 5. Create xcframework ────────────────────────────────────────────────────

echo "Creating xcframework..."

XCFRAMEWORK="$OUTPUT_DIR/$FRAMEWORK_NAME.xcframework"

# Remove existing xcframework
if [ -d "$XCFRAMEWORK" ]; then
    rm -rf "$XCFRAMEWORK"
fi

# Create xcframework with device and simulator frameworks
xcodebuild -create-xcframework \
    -framework "$DEVICE_FRAMEWORK_DIR" \
    -framework "$SIM_FRAMEWORK_DIR" \
    -output "$XCFRAMEWORK"

# Clean up temporary framework directories
rm -rf "target/ios-frameworks"

echo ""
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
