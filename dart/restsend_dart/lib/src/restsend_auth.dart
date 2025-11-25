import 'bridge_generated.dart/api.dart' as bridge;
import 'restsend_models.dart';
import 'runtime.dart';

class RestsendAuth {
  const RestsendAuth._();

  static Future<RestsendAuthInfo> loginWithPassword({
    required String endpoint,
    required String userId,
    required String password,
  }) async {
    await RestsendRuntime.ensureInitialized();
    final result = await bridge.loginWithPassword(
      endpoint: endpoint,
      userId: userId,
      password: password,
    );
    return RestsendAuthInfo.fromBridge(result);
  }
}
