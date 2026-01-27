# RestsendDart - Flutter 绑定

Restsend SDK 的 Flutter/Dart 绑定，使用 `flutter_rust_bridge` 实现。

## 快速开始

### macOS 桌面版（无需 Xcode）

这是最简单的方式，适合快速开发和测试：

```bash
# 从项目根目录执行
cd /Users/pi/workspace/rs/restsend-rs

# 1. 生成绑定代码并编译
./build_macos.sh

# 2. 运行示例应用
cd dart/restsend_dart/example
flutter clean
flutter pub get
flutter run -d macos
```

### iOS 模拟器（需要 Xcode）

```bash
# 1. 安装 Xcode（从 App Store）

# 2. 设置 Xcode 命令行工具
sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer

# 3. 编译 iOS 模拟器版本
cd /Users/pi/workspace/rs/restsend-rs
./build_ios_sim.sh

# 4. 运行应用
cd dart/restsend_dart/example
flutter clean
flutter pub get
cd ios && pod install && cd ..
flutter run
```

## 构建脚本说明

### build_macos.sh
- 编译 macOS arm64 版本
- 复制 dylib 到 Flutter 插件目录
- **不需要 Xcode**

### build_ios_sim.sh  
- 编译 iOS 模拟器 arm64 版本
- 创建 xcframework
- **需要 Xcode**

### build.dart
- 完整的自动化构建脚本
- 生成 Flutter Rust Bridge 绑定代码
- 编译所有平台（macOS、iOS、iOS 模拟器）
- 创建通用 xcframework
- **需要 Xcode 和所有 Rust targets**

## 开发工作流

### 修改 Rust 代码后

1. 重新生成绑定（如果修改了 API）：
```bash
flutter_rust_bridge_codegen generate \
  --rust-root crates/restsend-dart \
  --rust-input crate::api \
  --rust-output crates/restsend-dart/src/frb_generated.rs \
  --dart-output dart/restsend_dart/lib/src/bridge_generated.dart \
  --dart-entrypoint-class-name RestsendApi
```

2. 重新编译：
```bash
./build_macos.sh  # 或 ./build_ios_sim.sh
```

3. 重新运行 Flutter 应用：
```bash
cd dart/restsend_dart/example
flutter run
```

### 只修改 Dart/Flutter 代码

直接运行 `flutter run`，不需要重新编译 Rust 代码。

## 故障排除

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
