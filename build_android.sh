#!/bin/bash
set -e

echo "Building restsend-dart for Android..."

# Output directory
OUTPUT_DIR="dart/restsend_dart/android/src/main/jniLibs"
LIB_NAME="librestsend_dart.so"

# Required Rust targets for Android
ANDROID_TARGETS=(
    "aarch64-linux-android"
    "armv7-linux-androideabi"
    "x86_64-linux-android"
    "i686-linux-android"
)

# Check if cargo-ndk is installed (it registers as a cargo subcommand)
if ! cargo ndk --version &> /dev/null; then
    echo "Installing cargo-ndk..."
    cargo install cargo-ndk
fi

# Add missing Rust Android targets
echo "Checking Rust Android targets..."
INSTALLED_TARGETS=$(rustup target list --installed)
for target in "${ANDROID_TARGETS[@]}"; do
    if ! echo "$INSTALLED_TARGETS" | grep -q "$target"; then
        echo "  Adding Rust target: $target"
        rustup target add "$target"
    else
        echo "  ✓ $target"
    fi
done

# Check if Android NDK is available
if [ -z "$ANDROID_NDK_HOME" ] && [ -z "$NDK_HOME" ]; then
    # Try to auto-detect common NDK locations
    DETECTED_NDK=""
    # macOS: Android Studio default
    if [ -d "$HOME/Library/Android/sdk/ndk" ]; then
        DETECTED_NDK=$(ls -1 "$HOME/Library/Android/sdk/ndk" | sort -V | tail -1)
        if [ -n "$DETECTED_NDK" ]; then
            export ANDROID_NDK_HOME="$HOME/Library/Android/sdk/ndk/$DETECTED_NDK"
            echo "Auto-detected NDK: $ANDROID_NDK_HOME"
        fi
    fi
    # Linux: Android Studio default
    if [ -z "$DETECTED_NDK" ] && [ -d "$HOME/Android/Sdk/ndk" ]; then
        DETECTED_NDK=$(ls -1 "$HOME/Android/Sdk/ndk" | sort -V | tail -1)
        if [ -n "$DETECTED_NDK" ]; then
            export ANDROID_NDK_HOME="$HOME/Android/Sdk/ndk/$DETECTED_NDK"
            echo "Auto-detected NDK: $ANDROID_NDK_HOME"
        fi
    fi

    if [ -z "$ANDROID_NDK_HOME" ]; then
        echo "Error: ANDROID_NDK_HOME or NDK_HOME is not set and NDK could not be auto-detected."
        echo ""
        echo "Please install the Android NDK and set the environment variable:"
        echo "  export ANDROID_NDK_HOME=\$HOME/Library/Android/sdk/ndk/27.0.12077973  # macOS"
        echo "  export ANDROID_NDK_HOME=\$HOME/Android/Sdk/ndk/27.0.12077973           # Linux"
        echo ""
        echo "You can install the NDK via Android Studio > SDK Manager > SDK Tools > NDK (Side by side)"
        echo "or download it from: https://developer.android.com/ndk/downloads"
        exit 1
    fi
fi

# Set NDK path
NDK_PATH="${ANDROID_NDK_HOME:-$NDK_HOME}"
echo "Using NDK: $NDK_PATH"

# Verify NDK path exists
if [ ! -d "$NDK_PATH" ]; then
    echo "Error: NDK path does not exist: $NDK_PATH"
    exit 1
fi

# Clean output directory
rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"

# Build for all supported Android architectures using cargo-ndk
# -t: target ABI names
# -o: output directory for .so files (cargo-ndk creates ABI subdirs automatically)
echo ""
echo "Building for all Android architectures..."

cargo ndk \
    -t arm64-v8a \
    -t armeabi-v7a \
    -t x86_64 \
    -t x86 \
    -o "$OUTPUT_DIR" \
    build -p restsend-dart --release

# Verify output
echo ""
MISSING=0
for ABI in arm64-v8a armeabi-v7a x86_64 x86; do
    SO_PATH="$OUTPUT_DIR/$ABI/$LIB_NAME"
    if [ -f "$SO_PATH" ]; then
        SIZE=$(du -h "$SO_PATH" | cut -f1)
        echo "  ✅ $ABI/$LIB_NAME ($SIZE)"
    else
        echo "  ❌ $ABI/$LIB_NAME — NOT FOUND"
        MISSING=$((MISSING + 1))
    fi
done

if [ "$MISSING" -gt 0 ]; then
    echo ""
    echo "Error: $MISSING library/libraries are missing. Build may have failed."
    exit 1
fi

echo ""
echo "✅ Android libraries built successfully!"
echo ""
echo "Now you can run:"
echo "  cd dart/restsend_dart/example"
echo "  flutter clean"
echo "  flutter pub get"
echo "  flutter run -d <android-device-id>"
