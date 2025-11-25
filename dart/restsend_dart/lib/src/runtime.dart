import 'bridge_generated.dart/frb_generated.dart';

/// Ensures the native Rust library is loaded exactly once.
class RestsendRuntime {
  RestsendRuntime._();

  static bool _initialized = false;

  static Future<void> ensureInitialized() async {
    if (_initialized) {
      return;
    }
    await RustLib.init();
    _initialized = true;
  }
}
