restsend client sdk
=====

restsend的客户端SDK,基于rust编写, 提供android和ios的sdk实现, 需要对rust版本有要求, 如果版本低于 `1.72.0` 需要先 `rustup update` 更新到rust版本
查看rustc版本:
```shell
mpi@mpis-Mac-mini restsend-sdk % rustc --version
rustc 1.72.0 (5680fa18f 2023-08-23)
```

## 代码依赖和环境准备

- 需要`rust 1.72.0 (5680fa18f 2023-08-23)` 以上版本, 需要设置 [rsproxy.cn](https://rsproxy.cn) 加快编译速度
- ios 需要指定准备开发环境: (m1/m2 上使用)

    ```shell
    rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-darwin
    ```

- android 需要指定准备开发环境:
   android 需要安装ndk,并设置环境变量`NDK_HOME`指向ndk的安装目录
   1. 下载ndk, [ndk下载地址](https://developer.android.com/ndk/downloads?hl=zh-cn)
        - wget "<https://dl.google.com/android/repository/android-ndk-r25c-linux.zip?hl=zh-cn>"
        - 解压后得到ndk的文件目录， 修改 `.profile`文件，添加环境变量:

            ```shell
            export NDK_HOME=/home/xxx/ndk/android-ndk-r25c
            ```

   2. cargo增加android的toolchain

    ```shell
    cargo install cargo-ndk
    rustup target add aarch64-linux-android x86_64-linux-android
    ```

### 编译ios

只在M2的机器上测试通过, 请使用M2的机器编译
开发测试版本：模拟器
```ruby
# 在xcode工程的Podfile引入
pod 'restsendSdk', :path => '../restsend-sdk'
```

```shell
cargo build --target aarch64-apple-ios-sim --target x86_64-apple-darwin
# 构建xcframework, 本地的Podfile引入
./scripts/xcframework.sh --debug --dev
#
```

正式版本(release)：

```shell
    cargo build --target aarch64-apple-ios-sim --target aarch64-apple-ios --target x86_64-apple-darwin --release
    # 构建swift的framework, 并且会发布到rddoc.cn上
    ./scripts/xcframework.sh
```

### 编译Android

只有在m2的机器上测试通过,目前只支持arm的模拟器,x86_64的模拟器待定
先配置好NDK的目录指向ndk, 比如你下载到~/ndk目录中

```shell
export NDK_HOME=$HOME/ndk
cargo ndk -t arm64-v8a -t x86_64 -o ./jniLibs build --release

```

### 如何测试

- cargo开发阶段的测试, 是pc上的测试:

    ```shell
    cargo test
    ```

- python的demo， 在python-demo/demo.py, 需要先编译rust的代码才能执行

    ```shell
    cargo build
    cd python-demo
    RUST_LOG=debug python demo.py
    ```

- ios的测试 (会启动模拟器，不建议命令行启动, 建议用xcode上启动)

```shell
    cd ios-demo
    xcodebuild test -project restsend-demo.xcodeproj \
    -scheme restsend-demo \
    -destination 'platform=iOS Simulator,name=iPhone 14,OS=16.4'
```

- android的测试， 需要先启动模拟器:

```shell
# 需要有android的模拟设备
cd android-demo
./gradlew connectedDebugAndroidTest
```

### Android的配置

- 在`app/build.gradle` 增加jna的支持， 用aar，能把需要的.so自动编译进去

```gradle
dependencies {
    implementation 'net.java.dev.jna:jna:5.5.0@aar'
    ....
}
```

- 在`AndroidManifest.xml` 中增加存储和网络权限

```xml
    <uses-permission android:name="android.permission.INTERNET"></uses-permission>
    <application ...>
```

### 如何维护client.udl
udl是uniffi-rs的描述语言，保存client.udl之后，会通过build.rs自动生成bindings下的代码，包括kotlin, swift, python的代码
udl暂时没有linter, 如果出现client.udl保存后没有生成bindings对应的代码，需要在命令行手工运行cargo build, 看看错误的提示。
