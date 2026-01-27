import 'dart:io';

Future<void> main(List<String> arguments) async {
  await _runStep(
    name: 'Install flutter_rust_bridge_codegen',
    executable: 'cargo',
    arguments: const [
      'install',
      'flutter_rust_bridge_codegen',
      '--version',
      '2.11.1',
      '--locked',
    ],
  );

  const dartDir = 'dart/restsend_dart';

  await _runStep(
    name: 'Fetch Dart dependencies',
    executable: 'dart',
    arguments: const ['pub', 'get'],
    workingDirectory: dartDir,
  );

  await _runStep(
    name: 'Run build_runner',
    executable: 'dart',
    arguments: const [
      'run',
      'build_runner',
      'build',
      '--delete-conflicting-outputs',
    ],
    workingDirectory: dartDir,
  );

  await _runStep(
    name: 'Generate flutter_rust_bridge bindings',
    executable: 'flutter_rust_bridge_codegen',
    arguments: const [
      'generate',
      '--rust-root',
      'crates/restsend-dart',
      '--rust-input',
      'crate::api',
      '--rust-output',
      'crates/restsend-dart/src/frb_generated.rs',
      '--dart-output',
      'dart/restsend_dart/lib/src/bridge_generated.dart',
      '--dart-entrypoint-class-name',
      'RestsendApi',
    ],
  );

  await _runStep(
    name: 'Build restsend-dart crate (macOS arm64)',
    executable: 'cargo',
    arguments: const ['build', '-p', 'restsend-dart', '--release', '--target', 'aarch64-apple-darwin'],
  );

  await _runStep(
    name: 'Build restsend-dart crate (macOS x86_64)',
    executable: 'cargo',
    arguments: const ['build', '-p', 'restsend-dart', '--release', '--target', 'x86_64-apple-darwin'],
  );

  await _runStep(
    name: 'Build restsend-dart crate (iOS)',
    executable: 'cargo',
    arguments: const ['build', '-p', 'restsend-dart', '--release', '--target', 'aarch64-apple-ios'],
  );

  await _runStep(
    name: 'Build restsend-dart crate (iOS Simulator arm64)',
    executable: 'cargo',
    arguments: const ['build', '-p', 'restsend-dart', '--release', '--target', 'aarch64-apple-ios-sim'],
  );

  await _createXcframework();
  await _createMacosDylib();
}

Future<void> _createMacosDylib() async {
  stdout.writeln('\n>>> Creating macOS universal dylib');

  const dylibName = 'librestsend_dart.dylib';
  final macosArmDylib = 'target/aarch64-apple-darwin/release/$dylibName';
  final macosX64Dylib = 'target/x86_64-apple-darwin/release/$dylibName';

  // Create fat dylib for macOS
  final macosFatDir = 'target/macos-fat/release';
  await Directory(macosFatDir).create(recursive: true);
  final macosFatDylib = '$macosFatDir/$dylibName';
  
  await _runStep(
    name: 'Lipo macOS dylibs',
    executable: 'lipo',
    arguments: ['-create', macosArmDylib, macosX64Dylib, '-output', macosFatDylib],
  );

  // Copy to macos directory for non-CocoaPods builds
  await _runStep(
    name: 'Copy dylib to macos/',
    executable: 'cp',
    arguments: [macosFatDylib, 'dart/restsend_dart/macos/'],
  );
}

Future<void> _createXcframework() async {
  stdout.writeln('\n>>> Creating xcframework');

  const libName = 'librestsend_dart.a';
  final iosLib = 'target/aarch64-apple-ios/release/$libName';
  final iosSimLib = 'target/aarch64-apple-ios-sim/release/$libName';
  final macosArmLib = 'target/aarch64-apple-darwin/release/$libName';
  final macosX64Lib = 'target/x86_64-apple-darwin/release/$libName';

  // Create fat library for macOS
  final macosFatDir = 'target/macos-fat/release';
  await Directory(macosFatDir).create(recursive: true);
  final macosFatLib = '$macosFatDir/$libName';
  
  await _runStep(
    name: 'Lipo macOS libraries',
    executable: 'lipo',
    arguments: ['-create', macosArmLib, macosX64Lib, '-output', macosFatLib],
  );

  final output = 'dart/restsend_dart/restsend_dart_ffi.xcframework';
  if (await Directory(output).exists()) {
    await Directory(output).delete(recursive: true);
  }

  await _runStep(
    name: 'Xcodebuild create-xcframework',
    executable: 'xcodebuild',
    arguments: [
      '-create-xcframework',
      '-library', iosLib,
      '-library', iosSimLib,
      '-library', macosFatLib,
      '-output', output,
    ],
  );

  // Copy to ios and macos subdirectories for cocoapods
  await _runStep(
    name: 'Copy xcframework to ios/',
    executable: 'cp',
    arguments: ['-R', output, 'dart/restsend_dart/ios/'],
  );
  await _runStep(
    name: 'Copy xcframework to macos/',
    executable: 'cp',
    arguments: ['-R', output, 'dart/restsend_dart/macos/'],
  );
}

Future<void> _runStep({
  required String name,
  required String executable,
  required List<String> arguments,
  String? workingDirectory,
}) async {
  stdout.writeln('\n>>> $name');
  stdout.writeln('Running: $executable ${arguments.join(' ')}');
  if (workingDirectory != null) {
    stdout.writeln('Working directory: $workingDirectory');
  }

  final process = await Process.start(
    executable,
    arguments,
    workingDirectory: workingDirectory,
    runInShell: false,
  );

  await Future.wait([
    stdout.addStream(process.stdout),
    stderr.addStream(process.stderr),
  ]);

  final exitCode = await process.exitCode;
  if (exitCode != 0) {
    throw ProcessException(
      executable,
      arguments,
      'Step "$name" failed',
      exitCode,
    );
  }
}
