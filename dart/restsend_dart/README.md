# RestsendDart - Flutter 绑定

Restsend SDK 的 Flutter/Dart 绑定，使用 `flutter_rust_bridge` 实现。

## 前置要求

### 通用要求
- Flutter 3.24+ 和 Dart 3.5+
- Rust 1.85+ (`rustup update`)
- `flutter_rust_bridge_codegen` 2.11.1

### macOS 开发
```bash
# 安装 Rust target
rustup target add aarch64-apple-darwin x86_64-apple-darwin
```

### iOS 开发  
```bash
# 安装 Xcode（从 App Store）
# 设置命令行工具
sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer

# 安装 iOS Rust targets
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios

# 安装 CocoaPods（如果未安装）
sudo gem install cocoapods
```

## 快速开始

### 方式1: 自动构建（推荐）

从项目根目录执行完整构建：

```bash
cd /path/to/restsend-rs

# 生成 Flutter Rust Bridge 绑定并编译所有平台
dart run build.dart

# 运行 macOS 应用（最简单）
cd dart/restsend_dart/example
flutter run -d macos

# 或运行 iOS 模拟器
cd dart/restsend_dart/example
cd ios && pod install && cd ..
flutter run -d ios
```

### 方式2: 分步构建

#### macOS 桌面版（无需 Xcode）

这是最简单的方式，适合快速开发和测试：

```bash
# 从项目根目录执行
cd /path/to/restsend-rs

# 1. 生成绑定代码并编译 macOS 动态库
./build_macos.sh

# 2. 运行示例应用
cd dart/restsend_dart/example
flutter clean
flutter pub get
flutter run -d macos
```

#### iOS 模拟器（需要 Xcode）

**重要**: iOS 使用动态库（.dylib），不是静态库（.a）

```bash
# 1. 确保已安装 Xcode 和 Rust targets
sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer
rustup target add aarch64-apple-ios-sim x86_64-apple-ios

# 2. 从项目根目录编译 iOS 模拟器动态库（支持 arm64 + x86_64）
cd /path/to/restsend-rs
./build_ios_sim_dylib.sh

# 3. 运行应用
cd dart/restsend_dart/example
flutter clean
flutter pub get
cd ios && pod install && cd ..
flutter run -d ios
```

**注意**: 如果使用 `build_ios_sim.sh`（静态库版本），可能会遇到符号链接问题。推荐使用 `build_ios_sim_dylib.sh`。

## 构建脚本说明

### build_macos.sh
- 编译 macOS arm64 + x86_64 通用动态库
- 复制 dylib 到 Flutter 插件目录
- **不需要 Xcode**

### build_ios_sim_dylib.sh ⭐ (推荐)
- 编译 iOS 模拟器 arm64 + x86_64 通用动态库
- 适用于 Apple Silicon Mac (M1/M2/M3) 和 Intel Mac
- 动态库符号可在运行时被正确加载
- **需要 Xcode**

### build_ios_sim.sh (旧版，不推荐)
- 编译 iOS 模拟器静态库并创建 xcframework
- 可能遇到符号链接问题
- 仅保留用于参考

### build.dart (完整构建)
- 自动化构建脚本，包含所有步骤
- 生成 Flutter Rust Bridge 绑定代码
- 编译所有平台（macOS、iOS、iOS 模拟器）
- 创建通用 xcframework
- **需要 Xcode 和所有 Rust targets**

## 开发工作流

### 修改 Rust 代码后

1. 重新生成绑定（如果修改了 API）：
```bash
cd /path/to/restsend-rs

# 手动方式
flutter_rust_bridge_codegen generate \
  --rust-root crates/restsend-dart \
  --rust-input crate::api \
  --rust-output crates/restsend-dart/src/frb_generated.rs \
  --dart-output dart/restsend_dart/lib/src/bridge_generated.dart \
  --dart-entrypoint-class-name RestsendApi

# 或使用自动构建
dart run build.dart
```

2. 重新编译对应平台：
```bash
./build_macos.sh           # macOS
./build_ios_sim_dylib.sh   # iOS 模拟器
```

3. 重新运行 Flutter 应用：
```bash
cd dart/restsend_dart/example
flutter run -d macos  # 或 -d ios
```

### 只修改 Dart/Flutter 代码

直接运行 `flutter run`，使用热重载即可，不需要重新编译 Rust 代码。

## 故障排除

### iOS: 符号未找到错误

**错误**：`Failed to lookup symbol 'frb_get_rust_content_hash': symbol not found`

**原因**：使用了静态库但符号未正确链接

**解决**：
```bash
# 使用动态库版本
cd /path/to/restsend-rs
./build_ios_sim_dylib.sh

# 确保 runtime.dart 中使用 ExternalLibrary.open
# 确保 podspec 中使用 vendored_libraries
```

### iOS: xcframework 架构不匹配

**错误**：`Unable to find matching slice in 'ios-arm64 ios-arm64-simulator' for (arm64 x86_64)`

**原因**：xcframework 只包含 arm64，但 Xcode 同时需要 x86_64

**解决**：
```bash
# 确保安装了 x86_64 target
rustup target add x86_64-apple-ios

# 使用支持两种架构的构建脚本
./build_ios_sim_dylib.sh  # 已包含 lipo 合并
```

### 问题：SDK "iphonesimulator" cannot be located

**原因**：未安装 Xcode 或未正确配置

**解决**：
```bash
# 检查当前配置
xcode-select -p

# 如果显示 /Library/Developer/CommandLineTools，需要安装 Xcode 并执行：
sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer
```

### 问题：Failed to load dynamic library

**原因**：Rust 库未编译或路径不正确

**解决**：
1. 确认 `dart/restsend_dart/macos/librestsend_dart.dylib` 存在
2. 重新运行构建脚本：`./build_macos.sh`
3. 清理 Flutter 缓存：`cd dart/restsend_dart/example && flutter clean`

### 问题：error: no such command: `expand`

**原因**：缺少 cargo-expand

**解决**：
```bash
cargo install cargo-expand
```

### 问题：OpenSSL 相关错误

**原因**：交叉编译时 OpenSSL 配置问题

**解决**：项目已针对 macOS/iOS 平台优化，使用系统 Security.framework 而非 vendored OpenSSL

## 项目结构

```
restsend-rs/
├── crates/
│   └── restsend-dart/        # Rust FFI 代码
│       ├── src/
│       │   ├── api/          # FFI API 定义
│       │   └── frb_generated.rs  # 自动生成
│       └── Cargo.toml
├── dart/
│   └── restsend_dart/        # Dart 插件包
│       ├── lib/
│       │   ├── src/
│       │   │   ├── runtime.dart          # 库加载逻辑
│       │   │   └── bridge_generated.dart # 自动生成
│       │   └── restsend_dart.dart
│       ├── example/          # 示例应用
│       ├── ios/              # iOS 插件配置
│       ├── macos/            # macOS 插件配置
│       └── pubspec.yaml
├── build_macos.sh            # macOS 快速构建
├── build_ios_sim.sh          # iOS 模拟器构建
└── build.dart                # 完整构建脚本
```

## 依赖要求

### 必需
- Rust 1.85.0+ 
- Flutter 3.24+
- Dart 3.5+

### 可选（根据目标平台）
- **macOS 开发**：无特殊要求
- **iOS 开发**：Xcode 14+
- **Android 开发**：Android SDK + NDK

## 更多信息

查看 [iOS_BUILD.md](iOS_BUILD.md) 了解详细的 iOS 构建说明和故障排除。
