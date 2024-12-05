import 'package:integration_test/integration_test.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:restsend_dart/restsend_dart.dart';

const testEndpoint = "http://chat.ruzhila.cn";

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async => await RustLib.init());
  test('connect with callback', () async {
    final result = await signin(
        endpoint: testEndpoint, userId: "alice", password: "alice:demo");

    final client = await Client.newInstance(info: result);
    //client.
    //client.s
    // client.onconnected = () => {
    //   print("connected");
    // }
  });
}
