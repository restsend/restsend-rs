# 使用指南 - RestsendDart Flutter Example

## 当前状态 ✅

所有代码修复已完成，macOS 版本可以立即运行！

## 快速启动（macOS 桌面版）

```bash
# 1. 进入示例应用目录
cd /Users/pi/workspace/rs/restsend-rs/dart/restsend_dart/example

# 2. 清理并获取依赖
flutter clean
flutter pub get

# 3. 运行应用
flutter run -d macos
```

就这么简单！应用应该会启动并显示登录界面。

## 如果遇到库加载错误

如果看到 "Failed to load dynamic library" 错误，运行：

```bash
cd /Users/pi/workspace/rs/restsend-rs
./build_macos.sh
cd dart/restsend_dart/example
flutter run -d macos
```

## iOS 模拟器（需要 Xcode）

### 一次性设置

1. 从 Mac App Store 安装 Xcode（约 12GB）
2. 安装完成后，在终端运行：
```bash
sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer
xcodebuild -runFirstLaunch
```

### 构建并运行

```bash
# 1. 构建 iOS 模拟器框架
cd /Users/pi/workspace/rs/restsend-rs
./build_ios_sim.sh

# 2. 运行应用
cd dart/restsend_dart/example
flutter clean
flutter pub get
cd ios && pod install && cd ..
flutter run  # 自动选择 iOS 模拟器
```

## 开发工作流

### 只修改 Dart/Flutter 代码

直接运行 `flutter run`，支持热重载。

### 修改了 Rust 代码

```bash
# 重新编译 Rust 库
cd /Users/pi/workspace/rs/restsend-rs
./build_macos.sh  # 或 ./build_ios_sim.sh

# 重新运行应用（Dart 代码会自动重新加载）
cd dart/restsend_dart/example
flutter run
```

### 修改了 Rust API（添加/删除/修改函数）

```bash
# 1. 重新生成绑定代码
cd /Users/pi/workspace/rs/restsend-rs
flutter_rust_bridge_codegen generate \
  --rust-root crates/restsend-dart \
  --rust-input crate::api \
  --rust-output crates/restsend-dart/src/frb_generated.rs \
  --dart-output dart/restsend_dart/lib/src/bridge_generated.dart \
  --dart-entrypoint-class-name RestsendApi

# 2. 重新运行 build_runner
cd dart/restsend_dart
dart run build_runner build --delete-conflicting-outputs

# 3. 重新编译 Rust 库
cd ../..
./build_macos.sh

# 4. 运行应用
cd dart/restsend_dart/example
flutter run
```

## 应用功能

示例应用包含以下功能：

1. **登录界面**
   - 输入服务器端点
   - 输入用户 ID 和密码
   - 可选的自定义数据库路径

2. **会话列表**
   - 显示所有会话
   - 下拉刷新同步
   - 显示未读消息数

3. **聊天界面**
   - 加载历史消息
   - 发送文本消息
   - 实时接收新消息

## 调试技巧

### 查看 Rust 日志

在 Rust 代码中使用 `log::info!()`, `log::debug!()` 等，日志会输出到：
- **macOS**: 控制台应用（Console.app）
- **iOS**: Xcode 控制台

### 查看 Flutter 日志

```bash
flutter run -v  # 详细日志
```

### 清理所有缓存

```bash
cd dart/restsend_dart/example
flutter clean
rm -rf build/
rm -rf ios/Pods/
rm -rf macos/Pods/
flutter pub get
```

## 常见问题

### Q: 为什么推荐 macOS 版本？

A: macOS 桌面版：
- ✅ 不需要 Xcode
- ✅ 编译速度快
- ✅ 调试方便
- ✅ 开发体验好

iOS 模拟器虽然更接近真实设备，但需要 Xcode（12GB+），编译慢，且首次设置复杂。

### Q: macOS 和 iOS 版本有什么区别？

A: 对于这个 SDK，功能完全一致。主要区别是：
- UI 在 macOS 上更大，适合桌面
- iOS 模拟器更接近手机体验
- 底层都是同样的 Rust 代码和 Flutter 代码

### Q: 可以在 macOS 上开发，然后在 iOS 上测试吗？

A: 完全可以！建议的工作流：
1. 在 macOS 上开发和调试
2. 功能完成后安装 Xcode
3. 构建 iOS 版本进行最终测试

## 需要帮助？

- 查看 [README.md](../README.md) - 完整文档
- 查看 [iOS_BUILD.md](../iOS_BUILD.md) - iOS 详细说明
- 查看 [/Users/pi/workspace/rs/restsend-rs/SUMMARY.md](../../../SUMMARY.md) - 修复总结
