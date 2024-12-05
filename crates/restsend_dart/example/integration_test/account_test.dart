import 'package:integration_test/integration_test.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:restsend_dart/restsend_dart.dart';

const testEndpoint = "http://chat.ruzhila.cn";

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async => await RustLib.init());
  test('signin with bad password', () async {
    expect(
      () async {
        await signin(
            endpoint: testEndpoint, userId: "alice", password: "alice:123456");
      },
      throwsA(isA<Exception>().having(
          (e) => e.toString(), 'description', contains('invalid password'))),
    );
  });
  test('signin with password', () async {
    final result = await signin(
        endpoint: testEndpoint, userId: "alice", password: "alice:demo");
    expect(result, isNotNull);
  });
  test('signin with token', () async {
    final result = await signin(
        endpoint: testEndpoint, userId: "alice", password: "alice:demo");
    expect(result, isNotNull);

    final token = result.token;
    final result2 =
        await signin(endpoint: testEndpoint, userId: "alice", token: token);
    expect(result2, isNotNull);
  });
}
