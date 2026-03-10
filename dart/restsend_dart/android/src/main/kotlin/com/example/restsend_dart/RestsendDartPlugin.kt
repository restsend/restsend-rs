package com.example.restsend_dart

import io.flutter.embedding.engine.plugins.FlutterPlugin

/** RestsendDartPlugin */
class RestsendDartPlugin : FlutterPlugin {
    override fun onAttachedToBinding(flutterPluginBinding: FlutterPlugin.FlutterPluginBinding) {
        // No-op, just to satisfy the Flutter plugin system
        // The actual FFI is handled by flutter_rust_bridge
    }

    override fun onDetachedFromBinding(binding: FlutterPlugin.FlutterPluginBinding) {
        // No-op
    }
}
