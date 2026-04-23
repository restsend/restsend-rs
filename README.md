# RestSend Client SDK

RestSend is a secure instant-messaging platform (server in Go, client SDK in Rust) that targets Android, iOS, WASM, and desktop. The Rust core exposes a high-reliability protocol, sync callbacks, and full OpenAPI/Webhook support so that existing user systems can extend or embed the service with minimal effort.

- Benchmarked with 80%+ unit-test coverage and millions of concurrent connections per node.
- SDK crates live in `crates/` and power mobile bindings (`android-sdk`, `swift/`) plus WebAssembly (`crates/restsend-wasm`).

Try the [online demo](https://chat.ruzhila.cn?from=github) or reach out at `kui@fourz.cn`.

## Requirements

- Rust 1.85+ (`rustup update` if needed) and basic build dependencies (`clang`, `cmake`).
- Optional: [rsproxy.cn](https://rsproxy.cn) mirrors for faster builds inside China.
- Platform extras:
  - **iOS/macOS**: `rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-darwin` and Xcode 15+.
  - **Android**: Android NDK r25c+, set `NDK_HOME`, install `cargo-ndk`, and add `aarch64-linux-android`, `armv7-linux-androideabi`, `x86_64-linux-android` targets.
  - **WASM**: Node.js 20+, `wasm-pack`, `wasm-bindgen-cli`, `wasm-opt` from Binaryen.

## Quick Start

```shell
git clone https://github.com/restsend/restsend-rs.git
cd restsend-rs
cargo test                     # verify the Rust core
```

Choose a target build from the sections below.

## Backend Build & Deploy (Simple)

1. Build:

```shell
cargo build -p restsend-backend --release
```

2. Create a `.env` file at repository root (or export env vars directly):

```env
RS_ADDR=0.0.0.0:8080
RS_DATABASE_URL=sqlite://restsend-server.db?mode=rwc
RS_OPENAPI_PREFIX=/open
RS_API_PREFIX=/api
RS_OPENAPI_TOKEN=change-me
RS_LOG_FILE=logs/restsend-backend.log
RS_RUN_MIGRATIONS=true
RS_MIGRATE_ONLY=false
RS_WEBHOOK_TARGETS=
RS_WEBHOOK_TIMEOUT_SECS=10
RS_WEBHOOK_RETRIES=3
RS_EVENT_BUS_SIZE=1024
RS_MESSAGE_WORKERS=4
RS_MESSAGE_QUEUE_SIZE=1024
RS_PRESENCE_BACKEND=memory
RS_NODE_ID=node-a
RS_PRESENCE_TTL_SECS=90
RS_PRESENCE_HEARTBEAT_SECS=30
```

`restsend-backend` supports loading `.env` via `dotenvy`.

Presence options:

- `RS_PRESENCE_BACKEND=memory`: single-node in-memory presence.
- `RS_PRESENCE_BACKEND=db`: database-backed presence (cluster-friendly; multiple backend nodes share online state).

3. Run migrations only (optional):

```shell
RS_MIGRATE_ONLY=true cargo run -p restsend-backend --release
```

4. Start backend:

```shell
cargo run -p restsend-backend --release
```

Default health check: `GET /api/health`

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


# Commercial License Terms

RestSend is available under a dual-license model:

- MIT License (see `LICENSE`)
- Commercial License (this file)

## When Commercial License Is Required

If your production deployment serves **more than 1000 users**, you must obtain a commercial license from the copyright holder.

## Contact

For commercial licensing, contact: `kui@fourz.cn`

## Scope

Without a commercial agreement, usage above the 1000-user threshold is not authorized.

All other rights and obligations are governed by your signed commercial agreement.
