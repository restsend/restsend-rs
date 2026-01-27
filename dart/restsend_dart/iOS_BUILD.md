# iOS 构建指南

## 背景：为什么 iOS 需要动态库

在 iOS 开发中，使用 Flutter Rust Bridge 时，**必须使用动态库（.dylib）** 而不是静态库（.a）。

### 技术原因

1. **符号可见性**：
   - 静态库：符号在编译时链接，但 flutter_rust_bridge 在运行时通过 `dlsym` 动态查找符号
   - 动态库：符号在运行时可见，`dlsym` 可以正确查找到 FFI 函数

2. **链接时机**：
   - 静态库需要在编译时显式链接所有符号
   - 动态库允许运行时动态加载和符号解析

3. **Flutter FFI 工作方式**：
   ```dart
   // runtime.dart 中的实际代码
   ExternalLibrary.open('librestsend_dart.dylib')  // ✅ 运行时加载
   ```

### 常见错误示例

使用静态库时会遇到：
```
Failed to lookup symbol 'frb_get_rust_content_hash': symbol not found
```

这是因为静态库中的符号虽然存在，但在运行时无法通过 dlsym 找到。

## 构建 iOS 模拟器动态库

### 前置要求

```bash
# 1. 安装 Xcode（从 App Store，约 15GB）
# 2. 配置命令行工具
sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer

# 3. 验证安装
xcodebuild -version
# 应显示：Xcode 15.x 或更高版本

# 4. 安装 Rust targets
rustup target add aarch64-apple-ios-sim  # Apple Silicon Mac 模拟器
rustup target add x86_64-apple-ios       # Intel Mac 模拟器

# 5. 验证 target 安装
rustup target list | grep installed
# 应包含：
# aarch64-apple-ios-sim
# x86_64-apple-ios
```

### 构建通用动态库

使用 `build_ios_sim_dylib.sh` 脚本（已创建并验证）：

```bash
cd /path/to/restsend-rs

# 查看脚本内容
cat build_ios_sim_dylib.sh

# 执行构建（约 2-5 分钟）
./build_ios_sim_dylib.sh
```

### 脚本工作流程

```bash
#!/bin/bash

# 1. 生成 FFI 绑定（如果需要）
dart run build.dart

# 2. 编译 arm64 版本（Apple Silicon）
cd crates/restsend-dart
cargo build --release --target aarch64-apple-ios-sim

# 3. 编译 x86_64 版本（Intel Mac）
cargo build --release --target x86_64-apple-ios

# 4. 合并为通用二进制
lipo -create \
  ../../target/aarch64-apple-ios-sim/release/librestsend_dart.dylib \
  ../../target/x86_64-apple-ios/release/librestsend_dart.dylib \
  -output ../../dart/restsend_dart/ios/librestsend_dart.dylib

# 5. 验证架构
lipo -info ../../dart/restsend_dart/ios/librestsend_dart.dylib
# 应显示：Architectures in the fat file: ... are: x86_64 arm64
```

### 验证构建产物

```bash
# 检查文件是否存在
ls -lh dart/restsend_dart/ios/librestsend_dart.dylib

# 查看支持的架构
lipo -info dart/restsend_dart/ios/librestsend_dart.dylib
# 应输出：Architectures in the fat file: ... are: x86_64 arm64

# 检查符号是否存在
nm -g dart/restsend_dart/ios/librestsend_dart.dylib | grep frb_get_rust_content_hash
# 应看到符号地址

# 验证是否为动态库
file dart/restsend_dart/ios/librestsend_dart.dylib
# 应包含 "Mach-O universal binary with 2 architectures"
```

## 配置 Flutter 项目

### 1. CocoaPods 配置

`dart/restsend_dart/ios/restsend_dart.podspec`：

```ruby
Pod::Spec.new do |s|
  s.name             = 'restsend_dart'
  s.version          = '1.0.0'
  s.summary          = 'Restsend SDK for Flutter'
  s.homepage         = 'https://github.com/restsend/restsend-rs'
  s.license          = { :file => '../LICENSE' }
  s.author           = { 'Restsend' => 'dev@restsend.com' }
  
  s.source           = { :path => '.' }
  s.source_files     = 'Classes/**/*'
  s.public_header_files = 'Classes/**/*.h'
  
  # ⭐ 关键：使用 vendored_libraries
  s.vendored_libraries = 'librestsend_dart.dylib'
  
  s.dependency 'Flutter'
  s.platform = :ios, '12.0'
  s.pod_target_xcconfig = { 'DEFINES_MODULE' => 'YES' }
end
```

### 2. Runtime 配置

`dart/restsend_dart/lib/src/runtime.dart`：

```dart
import 'dart:ffi';
import 'dart:io';
import 'package:flutter_rust_bridge/flutter_rust_bridge.dart';

ExternalLibrary getRustLibrary() {
  if (Platform.isMacOS) {
    return ExternalLibrary.open('librestsend_dart.dylib');
  } else if (Platform.isIOS) {
    // ⭐ iOS 使用动态库
    return ExternalLibrary.open('librestsend_dart.dylib');
  } else if (Platform.isAndroid) {
    return ExternalLibrary.open('librestsend_dart.so');
  }
  throw UnsupportedError('Unsupported platform: ${Platform.operatingSystem}');
}
```

### 3. Pubspec 配置

`dart/restsend_dart/pubspec.yaml`：

```yaml
flutter:
  plugin:
    platforms:
      ios:
        pluginClass: RestsendDartPlugin
      macos:
        pluginClass: RestsendDartPlugin
```

## 运行应用

### 完整流程

```bash
# 1. 构建动态库
cd /path/to/restsend-rs
./build_ios_sim_dylib.sh

# 2. 清理并安装依赖
cd dart/restsend_dart/example
flutter clean
flutter pub get
cd ios && pod install && cd ..

# 3. 列出可用设备
flutter devices

# 4. 运行应用
flutter run -d ios
# 或指定设备 ID
flutter run -d <device-id>
```

### 预期输出

```
✓ Built build/ios/iphoneos/Runner.app (4.5s)
Launching lib/main.dart on iPhone 16e in debug mode...
Running Xcode build...
└─Compiling, linking and signing...                      4.5s
Xcode build done.                                        5.2s

✓ Flutter run on iPhone 16e completed successfully
```

### 验证 FFI 工作

在应用日志中应看到：
```
[INFO] RestsendApi initialized successfully
[INFO] Login attempt with endpoint: https://api.example.com
```

如果看到业务逻辑错误（如 "invalid password"），说明 FFI 已正常工作。

## 故障排除

### 错误 1: 符号未找到

**错误**：
```
Failed to lookup symbol 'frb_get_rust_content_hash': symbol not found
```

**原因**：使用了静态库或符号未导出

**解决**：
```bash
# 1. 确认使用 build_ios_sim_dylib.sh（不是 build_ios_sim.sh）
./build_ios_sim_dylib.sh

# 2. 检查 podspec
cat dart/restsend_dart/ios/restsend_dart.podspec | grep vendored_libraries
# 应显示：s.vendored_libraries = 'librestsend_dart.dylib'

# 3. 检查 runtime.dart
cat dart/restsend_dart/lib/src/runtime.dart | grep iOS -A 2
# 应显示：ExternalLibrary.open('librestsend_dart.dylib')

# 4. 验证符号存在
nm -g dart/restsend_dart/ios/librestsend_dart.dylib | grep frb_get_rust_content_hash
```

### 错误 2: 架构不匹配

**错误**：
```
Unable to find matching slice in 'ios-arm64 ios-arm64-simulator' for (arm64 x86_64)
```

**原因**：缺少 x86_64 架构（Intel Mac 需要）

**解决**：
```bash
# 1. 确保安装了 x86_64 target
rustup target add x86_64-apple-ios

# 2. 重新构建包含两种架构的通用库
./build_ios_sim_dylib.sh

# 3. 验证架构
lipo -info dart/restsend_dart/ios/librestsend_dart.dylib
# 应显示：x86_64 arm64
```

### 错误 3: SDK not found

**错误**：
```
SDK "iphonesimulator" cannot be located
```

**原因**：未安装 Xcode 或配置错误

**解决**：
```bash
# 1. 检查 Xcode 安装
xcode-select -p
# 应显示：/Applications/Xcode.app/Contents/Developer

# 2. 如果不正确，切换到 Xcode
sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer

# 3. 验证
xcodebuild -version
# 应显示：Xcode 15.x

# 4. 如果还没有 Xcode，需要从 App Store 下载（约 15GB）
```

### 错误 4: CocoaPods 警告

**警告**：
```
[!] `restsend_dart` does not specify a Swift version
[!] no license specified in `restsend_dart`
```

**解决**：
```bash
# 1. 创建 LICENSE 文件
touch dart/restsend_dart/LICENSE

# 2. 在 podspec 中添加 swift_version（可选）
# s.swift_version = '5.0'
```

## 开发工作流

### 快速迭代

1. **只修改 Dart 代码**：直接使用热重载，无需重新编译 Rust
   ```bash
   flutter run -d ios
   # 按 'r' 进行热重载
   ```

2. **修改 Rust 代码**：重新编译并重启
   ```bash
   ./build_ios_sim_dylib.sh
   cd dart/restsend_dart/example
   flutter run -d ios
   ```

3. **修改 FFI 接口**：重新生成绑定并编译
   ```bash
   dart run build.dart
   ./build_ios_sim_dylib.sh
   cd dart/restsend_dart/example
   flutter run -d ios
   ```

### 调试技巧

1. **查看完整日志**：
   ```bash
   flutter run -d ios -v
   ```

2. **查看 FFI 调用**：在 Dart 代码中添加日志
   ```dart
   print('Calling Rust function...');
   final result = await api.someFunction();
   print('Rust returned: $result');
   ```

3. **查看 Xcode 控制台**：
   - 打开 Xcode
   - Window → Devices and Simulators
   - 选择设备 → Open Console
   - 查看底层日志

4. **检查库加载**：
   ```dart
   try {
     final lib = getRustLibrary();
     print('Library loaded successfully');
   } catch (e) {
     print('Failed to load library: $e');
   }
   ```

## 最佳实践

1. **始终使用动态库**：iOS 上避免使用静态库和 xcframework
2. **构建通用二进制**：同时支持 arm64 和 x86_64 架构
3. **使用 CocoaPods**：简化依赖管理
4. **清理构建缓存**：遇到奇怪问题时运行 `flutter clean`
5. **验证符号导出**：构建后使用 `nm -g` 检查符号

## 参考资源

- [Flutter Rust Bridge 文档](https://cjycode.com/flutter_rust_bridge/)
- [iOS Dynamic Libraries](https://developer.apple.com/library/archive/documentation/DeveloperTools/Conceptual/DynamicLibraries/)
- [Rust FFI Guide](https://doc.rust-lang.org/nomicon/ffi.html)
- [CocoaPods Guides](https://guides.cocoapods.org/)

## 总结

✅ **成功配置清单**：
- [x] 安装 Xcode 和 Rust targets
- [x] 使用 `build_ios_sim_dylib.sh` 构建动态库
- [x] 配置 `restsend_dart.podspec` 使用 `vendored_libraries`
- [x] 配置 `runtime.dart` 使用 `ExternalLibrary.open()`
- [x] 运行 `pod install` 安装 CocoaPods 依赖
- [x] 使用 `flutter run -d ios` 启动应用
- [x] 验证 FFI 调用正常工作

🎯 **关键点**：
- iOS 必须使用动态库（.dylib）
- 需要支持 arm64 + x86_64 双架构
- 使用 CocoaPods vendored_libraries
- ExternalLibrary.open() 进行运行时加载
