# iOS 构建指南

## 产物说明

iOS 目录下有两套产物，各有用途：

| 文件 | 类型 | 用途 |
|---|---|---|
| `ios/restsend_dart_ffi.xcframework` | 静态库 `.a` | 编译期链接（CocoaPods `vendored_frameworks`） |
| `ios/librestsend_dart.dylib` | 动态库 `.dylib` | 模拟器运行时 FFI 加载（`ExternalLibrary.open()`） |

xcframework 包含两个切片：
- `ios-arm64` — 真机（real device）
- `ios-arm64_x86_64-simulator` — 模拟器（Apple Silicon + Intel Mac）

## 背景：为什么需要动态库

Flutter Rust Bridge 在运行时通过 `dlsym` 动态查找 FFI 符号，静态库的符号在运行时对 `dlsym` 不可见，所以**模拟器必须额外提供 `.dylib`**。

```dart
// runtime.dart — iOS 模拟器运行时加载
ExternalLibrary.open('librestsend_dart.dylib')
```

真机走静态 xcframework 编译链接，不需要 dylib。

### install name 问题

dylib 必须将 install name 设为 `@rpath/librestsend_dart.dylib`，否则 iOS 在运行时按绝对路径查找，找不到就报：

```
Failed to load dynamic library 'librestsend_dart.dylib': dlopen(...) tried: ...
'/private/var/containers/Bundle/.../Runner.app/librestsend_dart.dylib' (no such file)
```

构建脚本已通过 `install_name_tool -id "@rpath/..."` 自动修复此问题。

## 前置要求

```bash
# 1. 安装 Xcode（从 App Store，约 15GB）
# 2. 配置命令行工具
sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer

# 3. 验证安装
xcodebuild -version
# 应显示：Xcode 15.x 或更高版本

# 4. 安装 Rust targets
rustup target add aarch64-apple-ios          # 真机
rustup target add aarch64-apple-ios-sim      # Apple Silicon Mac 模拟器
rustup target add x86_64-apple-ios           # Intel Mac 模拟器

# 5. 验证
rustup target list --installed | grep ios
# 应包含以上三个 target
```

## 构建

### 真机 + 模拟器（推荐，一次搞定）

```bash
cd /path/to/restsend-rs
./build_ios.sh
```

产出：
- `dart/restsend_dart/ios/restsend_dart_ffi.xcframework`（真机 arm64 + 模拟器 arm64/x86_64）
- `dart/restsend_dart/ios/librestsend_dart.dylib`（模拟器运行时 dylib，install name 已修复）

### 仅模拟器（快速迭代）

```bash
./build_ios_sim_dylib.sh
```

产出：
- `dart/restsend_dart/ios/librestsend_dart.dylib`（模拟器运行时 dylib）

> xcframework 不会被更新，真机不可用。

## 验证构建产物

```bash
# xcframework 结构
find dart/restsend_dart/ios/restsend_dart_ffi.xcframework -type f
# 应包含：
#   ios-arm64/librestsend_dart.a
#   ios-arm64_x86_64-simulator/librestsend_dart.a

# 真机切片架构
lipo -info dart/restsend_dart/ios/restsend_dart_ffi.xcframework/ios-arm64/librestsend_dart.a
# arm64

# 模拟器切片架构
lipo -info dart/restsend_dart/ios/restsend_dart_ffi.xcframework/ios-arm64_x86_64-simulator/librestsend_dart.a
# x86_64 arm64

# dylib install name（必须是 @rpath/...，不能是绝对路径）
otool -D dart/restsend_dart/ios/librestsend_dart.dylib
# 应显示：@rpath/librestsend_dart.dylib

# dylib 架构
lipo -info dart/restsend_dart/ios/librestsend_dart.dylib
# x86_64 arm64

# 验证 FFI 符号存在
nm -g dart/restsend_dart/ios/librestsend_dart.dylib | grep frb_get_rust_content_hash
```

## 配置 Flutter 项目

### CocoaPods（`ios/restsend_dart.podspec`）

```ruby
Pod::Spec.new do |s|
  s.name             = 'restsend_dart'
  s.source_files     = 'Classes/**/*'
  s.dependency       'Flutter'
  s.platform         = :ios, '11.0'
  s.swift_version    = '5.0'
  s.pod_target_xcconfig = { 'DEFINES_MODULE' => 'YES', 'EXCLUDED_ARCHS[sdk=iphonesimulator*]' => 'i386' }

  # 静态库 xcframework（编译期链接，真机 + 模拟器）
  s.vendored_frameworks = 'restsend_dart_ffi.xcframework'
end
```

### Runtime（`lib/src/runtime.dart`）

```dart
if (Platform.isIOS) {
  // 模拟器运行时通过 @rpath 加载动态库
  externalLibrary = ExternalLibrary.open('librestsend_dart.dylib');
}
```

## 运行应用

```bash
cd dart/restsend_dart/example
flutter clean && flutter pub get
cd ios && pod install && cd ..

# 模拟器
flutter run -d <simulator-id>

# 真机
flutter run -d <device-id>
```

## 故障排除

### `Failed to load dynamic library 'librestsend_dart.dylib'`

**原因**：dylib 的 install name 是绝对路径，运行时找不到。

**解决**：
```bash
# 验证 install name
otool -D dart/restsend_dart/ios/librestsend_dart.dylib
# 如果不是 @rpath/...，重新构建
./build_ios_sim_dylib.sh
```

### `Failed to lookup symbol 'frb_get_rust_content_hash'`

**原因**：使用了静态库或符号未导出。

**解决**：确认 `runtime.dart` 中 iOS 走 `ExternalLibrary.open('librestsend_dart.dylib')`，而不是静态链接方式。

### 真机编译失败（xcframework 缺少 arm64 切片）

**原因**：只运行了 `build_ios_sim_dylib.sh`，xcframework 没有真机切片。

**解决**：
```bash
./build_ios.sh  # 完整构建，包含真机切片
```

### 架构不匹配

**原因**：缺少 x86_64 target（Intel Mac 模拟器需要）。

**解决**：
```bash
rustup target add x86_64-apple-ios
./build_ios.sh
```

### `SDK "iphonesimulator" cannot be located`

```bash
sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer
xcodebuild -version
```

## 开发工作流

| 场景 | 操作 |
|---|---|
| 只改 Dart 代码 | 直接热重载（`r`），无需重新编译 |
| 改 Rust 代码（模拟器） | `./build_ios_sim_dylib.sh` → `flutter run` |
| 改 Rust 代码（真机） | `./build_ios.sh` → `flutter run` |
| 改 FFI 接口 | `dart run build.dart` → `./build_ios.sh` → `flutter run` |

## 参考资源

- [Flutter Rust Bridge 文档](https://cjycode.com/flutter_rust_bridge/)
- [iOS Dynamic Libraries](https://developer.apple.com/library/archive/documentation/DeveloperTools/Conceptual/DynamicLibraries/)
- [Rust FFI Guide](https://doc.rust-lang.org/nomicon/ffi.html)
- [CocoaPods Guides](https://guides.cocoapods.org/)
