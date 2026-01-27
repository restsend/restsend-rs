# RestsendDart 构建与运行说明

## 当前状态

✅ **已完成的修复**：
1. 修复了 `runtime.dart` - 添加了对 iOS 和 Android 的支持
2. 修复了 `Cargo.toml` - 针对 iOS/macOS 平台优化了 OpenSSL 配置
3. 创建了 `build_macos.sh` - macOS 构建脚本（无需 Xcode）
4. 创建了 `build_ios_sim.sh` - iOS 模拟器构建脚本（需要 Xcode）

## 快速开始 - macOS 桌面版（推荐）

如果你只是想快速测试功能，在 macOS 桌面版运行**不需要 Xcode**：

```bash
cd /Users/pi/workspace/rs/restsend-rs

# 1. 编译 macOS 版本
./build_macos.sh

# 2. 运行 Flutter 应用
cd dart/restsend_dart/example
flutter clean
flutter pub get
flutter run -d macos
```

## iOS 模拟器版本（需要 Xcode）

### 前置条件

1. **安装 Xcode**：
   - 从 Mac App Store 下载并安装 Xcode
   - 安装完成后运行：
     ```bash
     sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer
     ```

2. **安装 Rust targets**：
   ```bash
   rustup target add aarch64-apple-ios-sim aarch64-apple-ios
   ```

3. **构建 iOS 模拟器版本**：
   ```bash
   cd /Users/pi/workspace/rs/restsend-rs
   ./build_ios_sim.sh
   ```

4. **运行 Flutter 应用**：
   ```bash
   cd dart/restsend_dart/example
   flutter clean
   flutter pub get
   cd ios && pod install && cd ..
   flutter run
   ```

### 方案 2：使用 GitHub Actions 预编译（暂时没有可用的预编译版本）

如果你不想安装 Xcode，可以：
1. 在有 Xcode 的机器上运行 `./build_ios_sim.sh`
2. 将生成的 `dart/restsend_dart/ios/restsend_dart_ffi.xcframework` 复制到你的机器

### 方案 3：仅在 macOS 桌面运行

如果只是要测试功能，可以在 macOS 桌面版运行：
```bash
cd dart/restsend_dart/example
flutter clean
flutter pub get  
flutter run -d macos
```

## 当前修复内容

1. **修复了 runtime.dart** - 添加了对 iOS 的支持
2. **修复了 Cargo.toml** - 针对 iOS/macOS 平台禁用了 OpenSSL vendored feature
3. **创建了 build_ios_sim.sh** - 简化的 iOS 模拟器构建脚本

## 构建脚本说明

### build_ios_sim.sh
只编译 iOS 模拟器版本（arm64），适合快速开发测试。

### build.dart
完整的构建脚本，编译所有平台（iOS、iOS模拟器、macOS），并创建通用的 xcframework。

## 故障排除

### 错误：SDK "iphonesimulator" cannot be located
**原因**：没有安装 Xcode 或 xcode-select 指向命令行工具
**解决**：
```bash
# 检查当前配置
xcode-select -p

# 应该显示：/Applications/Xcode.app/Contents/Developer
# 如果显示 /Library/Developer/CommandLineTools，则需要切换：
sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer
```

### 错误：Failed to load dynamic library 'restsend_dart.framework/restsend_dart'
**原因**：xcframework 没有被正确构建或复制到 iOS 项目目录
**解决**：
1. 确保运行了构建脚本
2. 确保 `dart/restsend_dart/ios/restsend_dart_ffi.xcframework` 存在
3. 运行 `cd dart/restsend_dart/example/ios && pod install`

### macOS 桌面版运行正常
如果 macOS 桌面版可以运行，说明：
- Rust 编译环境正常
- Flutter 配置正常
- 只是缺少 iOS SDK 支持

在这种情况下，建议先使用 macOS 桌面版进行开发和测试。
