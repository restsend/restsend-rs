# RestSend Client SDK

RestSend is a secure instant-messaging platform (server in Go, client SDK in Rust) that targets Android, iOS, Flutter, WASM, and desktop. The Rust core exposes a high-reliability protocol, sync callbacks, and full OpenAPI/Webhook support so that existing user systems can extend or embed the service with minimal effort.

- Benchmarked with 80%+ unit-test coverage and millions of concurrent connections per node.
- SDK crates live in `crates/` and power mobile bindings (`android-sdk`, `swift/`) plus WebAssembly (`crates/restsend-wasm`).
- A Flutter/Dart package is shipped under `dart/restsend_dart` for cross-platform UI apps.

Try the [online demo](https://chat.ruzhila.cn?from=github) or reach out at `kui@fourz.cn`.

## Requirements

- Rust 1.85+ (`rustup update` if needed) and basic build dependencies (`clang`, `cmake`).
- Optional: [rsproxy.cn](https://rsproxy.cn) mirrors for faster builds inside China.
- Platform extras:
  - **iOS/macOS**: `rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-darwin` and Xcode 15+.
  - **Android**: Android NDK r25c+, set `NDK_HOME`, install `cargo-ndk`, and add `aarch64-linux-android`, `armv7-linux-androideabi`, `x86_64-linux-android` targets.
  - **Flutter/Dart**: Flutter 3.24+, Dart 3.5+, `flutter_rust_bridge_codegen 2.11.1`.
  - **WASM**: Node.js 20+, `wasm-pack`, `wasm-bindgen-cli`, `wasm-opt` from Binaryen.

## Quick Start

```shell
git clone https://github.com/restsend/restsend-rs.git
cd restsend-rs
cargo test                     # verify the Rust core
```

Choose a target build from the sections below.

### iOS / macOS Swift SDK

1. Add the pod inside your Xcode project:
   ```ruby
   pod 'restsendSdk', :path => '../restsend-rs'
   ```
2. Build the Rust library for simulator/device:
   ```shell
   cargo build --release --target aarch64-apple-ios-sim --target aarch64-apple-ios --target x86_64-apple-darwin
   ```
3. Regenerate Swift bindings when the API changes:
   ```shell
   cargo run --release --bin bindgen -- --language swift [-p true]   # add -p true to publish the pod bundle
   ```

### Android / Kotlin SDK

```shell
export NDK_HOME=/path/to/android-ndk-r25c
cargo ndk -t arm64-v8a -t armeabi-v7a build --release
cargo run --release --bin bindgen -- --language kotlin
cargo ndk -t arm64-v8a -t armeabi-v7a -o ./jniLibs build --release   # copies .so into jniLibs/
cd android-sdk && ./gradlew assembleRelease
```

Add JNA (or your preferred loader) plus network/storage permissions in your Android app manifest when embedding the produced AAR.

### Flutter / Dart bindings

The package under `dart/restsend_dart` uses `flutter_rust_bridge`.

```shell
dart run build.dart   # runs all required steps from the repo root
```

The helper script installs/updates `flutter_rust_bridge_codegen`, installs Dart deps, runs `build_runner`, regenerates bindings, and finally builds the `restsend-dart` crate.

Run `flutter run` from `dart/restsend_dart/example` to try the demo UI.

### WebAssembly package

```shell
cargo install wasm-pack wasm-bindgen-cli
cd crates/restsend-wasm
npm install --force
npx playwright install webkit
npm run build
```

Ship the generated artifacts from `crates/restsend-wasm/pkg/` (plus `assets/`) in your web app.

## Testing & Demos

- `cargo test` exercises the core logic.
- `cargo run -p demo` launches the native desktop sample (requires GUI environment).
- `dart/restsend_dart/example` showcases the Flutter bindings end to end.

Need help or commercial support? Email `kui@fourz.cn`.