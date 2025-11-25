import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import 'src/conversation_list_screen.dart';
import 'src/demo_controller.dart';
import 'src/login_screen.dart';

void main() {
  WidgetsFlutterBinding.ensureInitialized();
  runApp(const RestsendDemoApp());
}

class RestsendDemoApp extends StatelessWidget {
  const RestsendDemoApp({super.key});

  @override
  Widget build(BuildContext context) {
    return ChangeNotifierProvider(
      create: (_) => DemoController(),
      child: MaterialApp(
        title: 'Restsend Demo',
        theme: ThemeData(
          colorScheme: ColorScheme.fromSeed(seedColor: Colors.indigo),
          useMaterial3: true,
        ),
        home: const DemoHome(),
      ),
    );
  }
}

class DemoHome extends StatelessWidget {
  const DemoHome({super.key});

  @override
  Widget build(BuildContext context) {
    final controller = context.watch<DemoController>();
    switch (controller.phase) {
      case DemoPhase.unauthenticated:
        return const LoginScreen();
      case DemoPhase.connecting:
        return const _LoadingScreen(message: 'Connecting…');
      case DemoPhase.ready:
        return const ConversationListScreen();
      case DemoPhase.error:
        return const LoginScreen();
    }
  }
}

class _LoadingScreen extends StatelessWidget {
  const _LoadingScreen({required this.message});

  final String message;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const CircularProgressIndicator(),
            const SizedBox(height: 16),
            Text(message),
          ],
        ),
      ),
    );
  }
}
