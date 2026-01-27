import 'dart:io';
import 'package:flutter/foundation.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';
import 'bridge_generated.dart/frb_generated.dart';

/// Ensures the native Rust library is loaded exactly once.
class RestsendRuntime {
  RestsendRuntime._();

  static bool _initialized = false;

  static Future<void> ensureInitialized() async {
    if (_initialized) {
      return;
    }

    ExternalLibrary? externalLibrary;
    if (!kIsWeb) {
      if (Platform.isIOS) {
        // iOS uses the framework bundled in the app
        // The framework is loaded automatically, no need to specify path
        externalLibrary = ExternalLibrary.open('restsend_dart_ffi.framework/restsend_dart_ffi');
      } else if (Platform.isMacOS) {
        try {
          externalLibrary =
              ExternalLibrary.open('restsend_dart_ffi.framework/restsend_dart_ffi');
        } catch (_) {
          final String executableDir =
              File(Platform.resolvedExecutable).parent.path;
          final String localDylibPath =
              '$executableDir/librestsend_dart.dylib';

          try {
            externalLibrary = ExternalLibrary.open(localDylibPath);
          } catch (e) {
            try {
              externalLibrary = ExternalLibrary.open('librestsend_dart.dylib');
            } catch (e2) {
              debugPrint('Failed to load restsend_dart_ffi: $e\n$e2');
              rethrow;
            }
          }
        }
      } else if (Platform.isAndroid) {
        // Android loads the library automatically from jniLibs
        externalLibrary = ExternalLibrary.open('librestsend_dart.so');
      } else if (Platform.isLinux || Platform.isWindows) {
        final dylibName = Platform.isWindows ? 'restsend_dart.dll' : 'librestsend_dart.so';
        externalLibrary = ExternalLibrary.open(dylibName);
      }
    }

    await RestsendApi.init(externalLibrary: externalLibrary);
    _initialized = true;
  }
}
