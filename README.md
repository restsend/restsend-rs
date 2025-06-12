restsend client sdk
=====

RestSend is a secure instant messaging system, with the server written in Go:
- Unit test coverage is over `80%`
- Supports millions of concurrent connections on a single machine and supports cluster deployment
- High-reliability protocol design, supporting reliable message transmission
- Based on synchronous protocol design
- Complete `OpenAPI` and `Webhook` support, the server supports various forms of extensions: 
- Avatar generation
- Highly extensible account and permission system, no need to migrate existing user systems
- Flow control and content detection
- SDK is written in `Rust`, providing SDK implementations for `Android`, `iOS`, and `WASM`, capable of rendering `100,000` sessions on a single machine
- Code can be managed with `go module` and can be directly embedded into existing Go projects

For testing, you can visit the [online demo](https://chat.ruzhila.cn?from=github) or contact me at: kui@fourz.cn

The RestSend client SDK is written in Rust, providing SDK implementations for Android and iOS. It requires a Rust version of at least `1.85.0`. If the version is lower, please update Rust using `rustup update`.
Check the Rust version:

```shell
mpi@mpis-Mac-mini restsend-sdk % rustc --version
rustc 1.85.0 (4d91de4e4 2025-02-17)
```

## Code Dependencies and Environment Preparation

- Requires `rust 1.72.0 (5680fa18f 2023-08-23)` or higher, and setting up [rsproxy.cn](https://rsproxy.cn) to speed up compilation
- For iOS, prepare the development environment (for M1/M2):

        ```shell
        rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-darwin
        ```

- For Android, prepare the development environment:
     Install NDK and set the `NDK_HOME` environment variable to point to the NDK installation directory.
     1. Download NDK from [NDK download page](https://developer.android.com/ndk/downloads?hl=en)
       - wget "<https://dl.google.com/android/repository/android-ndk-r25c-linux.zip?hl=en>"
       - Extract the NDK directory and modify the `.profile` file to add the environment variable:
        ```shell
        export NDK_HOME=/home/xxx/ndk/android-ndk-r25c
        ```

     2. Add Android toolchain to cargo
        ```shell
        cargo install cargo-ndk
        rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android armv7-linux-androideabi
        ```

### Compile for iOS

Tested only on M2 machines, please use an M2 machine for compilation.

- Development test version: Simulator

        ```ruby
        # Add to Podfile in Xcode project
        pod 'restsendSdk', :path => '../restsend-rs'
        ```

  1. compile the Rust library
        ```shell
        cargo build --target aarch64-apple-ios-sim
        # For x86 Mac machines
        cargo build  --target x86_64-apple-darwin 
        ```
  1. Compile the Swift binding code

        ```shell
        cargo run --bin bindgen -- --language swift
        ```

- Release version:

   ```shell
   cargo build --release --target aarch64-apple-ios-sim --target aarch64-apple-ios --target x86_64-apple-darwin

   cargo run --release --bin bindgen -- --language swift
   ```
   To release the official pod version:

   ```shell
    cargo run --release --bin bindgen -- --language swift -p true
   ```

### Compile for Wasm

It is recommended to use Node version 20 or above.

First, manually download the wasm-opt tool from binaryen and place it in the PATH. Download from: <https://github.com/WebAssembly/binaryen/releases/>
After extraction, place wasm-opt in the PATH, for example, in the `/usr/local/bin` directory.

```shell
 cargo install wasm-pack wasm-bindgen-cli
 cd crates/restsend-wasm
 npm install --force
 
 # For Linux/WSL, install dependencies first
 # npx playwright install-deps
 
 npx playwright install webkit
 npm run build
```

### Compile for Android

Tested only on M2 machines, currently only supports ARM simulators, x86_64 simulators are pending.
First, configure the NDK directory to point to the NDK, for example, if downloaded to ~/ndk directory:

```shell
export NDK_HOME=$HOME/ndk
cargo ndk -t arm64-v8a -t armeabi-v7a build --release
cargo run --release --bin bindgen -- --language kotlin 
cargo ndk -t arm64-v8a -t armeabi-v7a -o ./jniLibs build --release

cd android-sdk
# test
# gradle connectedAndroidTest

gradle assembleRelease

# for Publish aar
gradle buildAndDeploy
```

### How to Test

- For cargo development phase testing, test on PC:
  ```shell
  cargo test
  ```

- Rust version demo requires a GUI environment, such as Windows/Mac to run
   ```shell
   cargo run -p demo
   ```

### Android Configuration

- In `app/build.gradle`, add JNA support using AAR to automatically compile the required .so files

        ```gradle
        dependencies {
                implementation 'net.java.dev.jna:jna:5.16.0@aar'
                ....
        }
        ```

- In `AndroidManifest.xml`, add storage and network permissions

        ```xml
                <uses-permission android:name="android.permission.INTERNET"></uses-permission>
                <application ...>
        ```