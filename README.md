Restsend Client SDK
=======

## Introduction
Restsend is a secure embeding instant messaging system. This is the client SDK of Restsend, which is written in Rust. 
Restsend is a team that provides IM/RTC API services. We are committed to implementing reliable communication services.

Demo: [demo page](https://chat.ruzhila.cn?from=github)
[中文文档](README.cn.md) 

## Code dependency and environment preparation
- rust 1.72.0 (5680fa18f 2023-08-23) or above
    > For china mainland developer: You can use [rsproxy.cn](https://rsproxy.cn) to speed up the compilation
- iOS development environment preparation: (Only for M1/M2 machine)
```
    rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-darwin
```
- Android development environment preparation:
    - Download ndk, [ndk download](https://developer.android.com/ndk/downloads)
        - wget "<https://dl.google.com/android/repository/android-ndk-r25c-linux.zip>"
        - unzip the ndk file, modify `.profile` file, add environment variable:
            ```shell
            export NDK_HOME=/home/YOUR/ndk/android-ndk-r25c
            ```
    - Add android toolchain to cargo
        ```shell
        cargo install cargo-ndk
        rustup target add aarch64-linux-android x86_64-linux-android
        ```

## How to build
### iOS build
- Debug version: simulator
```
    cargo build --target aarch64-apple-ios-sim
    # if you are using x86 machine
    cargo build  --target x86_64-apple-darwin 

    # build swift binding code
    cargo run --target aarch64-apple-ios-sim --bin bindgen -- --language swift
```
- Release version: device
```
    cargo build --target aarch64-apple-ios
    # build swift binding code
    cargo run --target aarch64-apple-ios --bin bindgen -- --language swift
```

### iOS Demo
```
    git clone https://github.com/restsend/restsend-swift.git
    cd restsend-swift
    pod install
```

### Web build
```
    cd crates/restsend-wasm
    # test rust library, the outout dir is `restsend-rs/crates/restsend-wasm/pkg`
    npm run test
```
Build the wasm library and js binding code, the outout dir is `restsend-rs/js`
```
    cd crates/restsend-wasm
    npm run dist
```
#### Show the web demo
```
    cd crates/restsend-wasm
    npm run dev
```