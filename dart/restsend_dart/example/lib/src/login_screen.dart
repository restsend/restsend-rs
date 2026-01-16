import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:restsend_dart/restsend_dart.dart';

import 'demo_controller.dart';

class LoginScreen extends StatefulWidget {
  const LoginScreen({super.key});

  @override
  State<LoginScreen> createState() => _LoginScreenState();
}

class _LoginScreenState extends State<LoginScreen> {
  final _formKey = GlobalKey<FormState>();
  final _endpointController = TextEditingController(text: 'https://chat.ruzhila.cn');
  final _userIdController = TextEditingController(text: 'bob');
  final _passwordController = TextEditingController(text: 'bob');
  final _nameController = TextEditingController();
  final _avatarController = TextEditingController();
  final _dbPathController = TextEditingController();
  final _dbNameController = TextEditingController(text: 'restsend_demo');

  bool _obscurePassword = true;

  @override
  void dispose() {
    _endpointController.dispose();
    _userIdController.dispose();
    _passwordController.dispose();
    _nameController.dispose();
    _avatarController.dispose();
    _dbPathController.dispose();
    _dbNameController.dispose();
    super.dispose();
  }

  Future<void> _submit() async {
    final form = _formKey.currentState;
    if (form == null || !form.validate()) {
      return;
    }
    final controller = context.read<DemoController>();
    final endpoint = _endpointController.text.trim();
    final userId = _userIdController.text.trim();
    final password = _passwordController.text.trim();
    final options = _dbPathController.text.trim().isEmpty &&
            _dbNameController.text.trim().isEmpty
        ? null
        : RestsendClientOptions(
            rootPath: _dbPathController.text.trim().isEmpty
                ? null
                : _dbPathController.text.trim(),
            dbName: _dbNameController.text.trim().isEmpty
                ? null
                : _dbNameController.text.trim(),
          );
    try {
      final auth = await RestsendAuth.loginWithPassword(
        endpoint: endpoint,
        userId: userId,
        password: password,
      );
      final enrichedAuth = RestsendAuthInfo(
        endpoint: auth.endpoint,
        userId: auth.userId,
        token: auth.token,
        name: _nameController.text.trim().isEmpty
            ? auth.name
            : _nameController.text.trim(),
        avatar: _avatarController.text.trim().isEmpty
            ? auth.avatar
            : _avatarController.text.trim(),
        isStaff: auth.isStaff,
        isCrossDomain: auth.isCrossDomain,
      );
      await controller.login(auth: enrichedAuth, options: options);
    } catch (err, stack) {
      debugPrint('Login failed: $err');
      debugPrintStack(stackTrace: stack);
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Login failed: $err')),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final controller = context.watch<DemoController>();
    final isLoading = controller.phase == DemoPhase.connecting;
    final errorText = controller.errorMessage;
    return Scaffold(
      appBar: AppBar(
        title: const Text('Restsend Demo Login'),
      ),
      body: SafeArea(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(16),
          child: Form(
            key: _formKey,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                const Text(
                  'Enter your endpoint, user ID, and password to connect to the Restsend SDK.',
                ),
                const SizedBox(height: 16),
                TextFormField(
                  controller: _endpointController,
                  decoration: const InputDecoration(
                    labelText: 'Endpoint',
                    hintText: 'https://api.example.com',
                  ),
                  validator: (value) {
                    if (value == null || value.trim().isEmpty) {
                      return 'Endpoint is required';
                    }
                    return null;
                  },
                ),
                const SizedBox(height: 12),
                TextFormField(
                  controller: _userIdController,
                  decoration: const InputDecoration(
                    labelText: 'User ID',
                  ),
                  validator: (value) {
                    if (value == null || value.trim().isEmpty) {
                      return 'User ID is required';
                    }
                    return null;
                  },
                ),
                const SizedBox(height: 12),
                TextFormField(
                  controller: _passwordController,
                  decoration: InputDecoration(
                    labelText: 'Password',
                    suffixIcon: IconButton(
                      icon: Icon(
                        _obscurePassword
                            ? Icons.visibility
                            : Icons.visibility_off,
                      ),
                      onPressed: () {
                        setState(() {
                          _obscurePassword = !_obscurePassword;
                        });
                      },
                    ),
                  ),
                  obscureText: _obscurePassword,
                  validator: (value) {
                    if (value == null || value.trim().isEmpty) {
                      return 'Password is required';
                    }
                    return null;
                  },
                ),
                const SizedBox(height: 12),
                TextFormField(
                  controller: _nameController,
                  decoration: const InputDecoration(
                    labelText: 'Display name (optional)',
                  ),
                ),
                const SizedBox(height: 12),
                TextFormField(
                  controller: _avatarController,
                  decoration: const InputDecoration(
                    labelText: 'Avatar URL (optional)',
                  ),
                ),
                const SizedBox(height: 12),
                TextFormField(
                  controller: _dbPathController,
                  decoration: const InputDecoration(
                    labelText: 'Local DB path (optional)',
                  ),
                ),
                const SizedBox(height: 12),
                TextFormField(
                  controller: _dbNameController,
                  decoration: const InputDecoration(
                    labelText: 'Local DB name (optional)',
                  ),
                ),
                const SizedBox(height: 24),
                FilledButton(
                  onPressed: isLoading ? null : _submit,
                  child: isLoading
                      ? const SizedBox(
                          height: 16,
                          width: 16,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      : const Text('Connect'),
                ),
                if (errorText != null) ...[
                  const SizedBox(height: 12),
                  Text(
                    errorText,
                    style: TextStyle(
                      color: Theme.of(context).colorScheme.error,
                    ),
                  ),
                ],
              ],
            ),
          ),
        ),
      ),
    );
  }
}
