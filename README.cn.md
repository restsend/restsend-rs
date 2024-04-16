restsend client sdk
=====

Restsend 是一个安全的即时通讯系统, 服务端用go编写：
- 单元测试覆盖率达到`80%`以上
- 支持单机百万的并发连接, 并且支持集群部署
- 高可靠协议设计，支持消息的可靠传输
- 基于同步协议设计
- 完整的`OpenAPI`和`Webhook`支持，服务端支持各种形式的扩展：
  - 头像生成
  - 账号和权限高度可扩展，现有的用户体系无需迁移
  - 流控和内容检测
- SDK基于`Rust`编写，提供`Android`和`iOS`、`WASM`的SDK实现, 单机能实现`10万`会话的渲染
- 代码可以用`go module`管理，可以直接嵌入到现有的go项目中

如果需要测试和体验，可以访问[在线demo](https://chat.ruzhila.cn?from=github) 也可以联系我：shenjint@fourz.cn


restsend的客户端SDK,基于rust编写, 提供android和ios的sdk实现, 对rust版本有要求, 如果版本低于 `1.74.0` 需要先 `rustup update` 更新到rust版本
查看rustc版本:

```shell
mpi@mpis-Mac-mini restsend-sdk % rustc --version
rustc 1.77.2 (25ef9e3d8 2024-04-09)
```

## 代码依赖和环境准备

- 需要`rust 1.72.0 (5680fa18f 2023-08-23) ` 以上版本, 需要设置 [rsproxy.cn](https://rsproxy.cn) 加快编译速度
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

- 开发测试版本：模拟器

    ```ruby
    # 在xcode工程的Podfile引入
    pod 'restsendSdk', :path => '../restsend-rs'
    ```

    1. 先编译rust的库

    ```shell
    cargo build --target aarch64-apple-ios-sim
    # 如果是x86的mac机器
    cargo build  --target x86_64-apple-darwin 
    ```

    1. 编译swift的绑定代码

    ```shell
    cargo run --bin bindgen -- --language swift
    ```

- 正式版本(release)：

    ```shell
    cargo build --release --target aarch64-apple-ios-sim --target aarch64-apple-ios --target x86_64-apple-darwin

    cargo run --release --bin bindgen -- --language swift
    ```

    如果需要发布正式的pod版本

    ```shell
    cargo run --release --bin bindgen -- --language swift -p true
    ```

### 编译Wasm版本

建议Node版本使用20以上版本

需要先手工下载binaryen的wasm-opt工具,并放到PATH中, 下载地址: <https://github.com/WebAssembly/binaryen/releases/>
解压后, 把wasm-opt放到PATH中, 比如放到`/usr/local/bin`目录中

```shell
 cargo install wasm-pack wasm-bindgen-cli
 cd crates/restsend-wasm
 npm install --force
 
 # Linux/WSL 需要先安装依赖
 # npx playwright install-deps
 
 npx playwright install webkit
 npm run build
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

- rust 版本的demo需要有GUI的环境,比如Windows/Mac才能运行

    ```shell
    cargo run -p demo
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
