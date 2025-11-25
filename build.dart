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
    name: 'Build restsend-dart crate',
    executable: 'cargo',
    arguments: const ['build', '-p', 'restsend-dart', '--release'],
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
