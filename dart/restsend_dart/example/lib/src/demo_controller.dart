import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:restsend_dart/restsend_dart.dart';

enum DemoPhase { unauthenticated, connecting, ready, error }

class SyncProgress {
  const SyncProgress({
    required this.updatedAt,
    required this.count,
    required this.total,
    this.lastRemovedAt,
  });

  final String updatedAt;
  final String? lastRemovedAt;
  final int count;
  final int total;

  bool get isFinished => total == 0 ? false : count >= total;
}

class DemoController extends ChangeNotifier {
  DemoPhase _phase = DemoPhase.unauthenticated;
  String? _errorMessage;
  RestsendClient? _client;
  RestsendAuthInfo? _authInfo;
  RestsendClientOptions? _clientOptions;
  StreamSubscription<RestsendClientEvent>? _eventSub;
  List<RestsendConversation> _conversations = const [];
  bool _isLoadingConversations = false;
  bool _isSyncing = false;
  SyncProgress? _syncProgress;

  DemoPhase get phase => _phase;
  String? get errorMessage => _errorMessage;
  List<RestsendConversation> get conversations => _conversations;
  bool get isLoadingConversations => _isLoadingConversations;
  bool get isSyncingConversations => _isSyncing;
  SyncProgress? get syncProgress => _syncProgress;
  RestsendClient? get client => _client;
  String? get currentUserId => _authInfo?.userId;
  bool get isAuthenticated => _client != null;

  Future<void> login({
    required RestsendAuthInfo auth,
    RestsendClientOptions? options,
  }) async {
    await logout();
    _phase = DemoPhase.connecting;
    _errorMessage = null;
    _authInfo = auth;
    _clientOptions = options;
    notifyListeners();
    try {
      final client = await RestsendClient.create(auth: auth, options: options);
      await client.connect();
      _client = client;
      _phase = DemoPhase.ready;
      _eventSub = client.events().listen(
        _handleEvent,
        onError: (Object err, StackTrace stack) {
          _errorMessage = 'Event stream error: $err';
          notifyListeners();
        },
      );
      notifyListeners();
      await loadConversations(refresh: true);
    } catch (err, stack) {
      debugPrint('DemoController login error: $err');
      debugPrintStack(stackTrace: stack);
      _errorMessage = 'Login failed: $err';
      _phase = DemoPhase.error;
      notifyListeners();
    }
  }

  Future<void> reconnect() async {
    final auth = _authInfo;
    if (auth == null) return;
    await login(auth: auth, options: _clientOptions);
  }

  Future<void> logout() async {
    _eventSub?.cancel();
    _eventSub = null;
    final client = _client;
    if (client != null) {
      await client.shutdown();
    }
    _client = null;
    _conversations = const [];
    _isLoadingConversations = false;
    _isSyncing = false;
    _syncProgress = null;
    if (_phase != DemoPhase.unauthenticated) {
      _phase = DemoPhase.unauthenticated;
    }
    notifyListeners();
  }

  Future<void> loadConversations({bool refresh = false}) async {
    final client = _client;
    if (client == null) return;
    if (_isLoadingConversations && !refresh) {
      return;
    }
    _isLoadingConversations = true;
    notifyListeners();
    try {
      final page = await client.listConversations(limit: 100);
      _conversations = List<RestsendConversation>.from(page.items)
        ..sort(
          (a, b) => b.updatedAt.compareTo(a.updatedAt),
        );
    } catch (err) {
      _errorMessage = 'Failed to load conversations: $err';
    } finally {
      _isLoadingConversations = false;
      notifyListeners();
    }
  }

  Future<void> syncConversations() async {
    final client = _client;
    if (client == null || _isSyncing) {
      return;
    }
    _isSyncing = true;
    _syncProgress = null;
    notifyListeners();
    try {
      await client.syncConversations(
        options: const RestsendSyncConversationsOptions(
          syncLogs: false,
          limit: 100,
        ),
      );
    } catch (err) {
      _errorMessage = 'Sync failed: $err';
      _isSyncing = false;
      notifyListeners();
    }
  }

  void _handleEvent(RestsendClientEvent event) {
    event.when(
      connected: () {
        if (_phase != DemoPhase.ready) {
          _phase = DemoPhase.ready;
          notifyListeners();
        }
      },
      connecting: () {
        _phase = DemoPhase.connecting;
        notifyListeners();
      },
      tokenExpired: (reason) {
        _errorMessage = 'Token expired: $reason';
        notifyListeners();
      },
      netBroken: (reason) {
        _errorMessage = 'Network error: $reason';
        notifyListeners();
      },
      kickedOffByOtherClient: (reason) {
        _errorMessage = 'Signed in elsewhere: $reason';
        notifyListeners();
      },
      conversationsUpdated: (items, total) {
        _mergeConversations(items);
      },
      conversationRemoved: (topicId) {
        _conversations = _conversations
            .where((conversation) => conversation.topicId != topicId)
            .toList(growable: false);
        notifyListeners();
      },
      messageReceived: (topicId, message, status) {
        unawaited(loadConversations(refresh: true));
      },
      syncConversationsProgress: (updatedAt, lastRemovedAt, count, total) {
        _syncProgress = SyncProgress(
          updatedAt: updatedAt,
          lastRemovedAt: lastRemovedAt,
          count: count,
          total: total,
        );
        _isSyncing = !(_syncProgress?.isFinished ?? false);
        notifyListeners();
        if (!_isLoadingConversations) {
          unawaited(loadConversations(refresh: true));
        }
      },
      syncConversationsFailed: (message) {
        _isSyncing = false;
        _syncProgress = null;
        _errorMessage = 'Sync failed: $message';
        notifyListeners();
      },
    );
  }

  void _mergeConversations(List<RestsendConversation> updates) {
    if (updates.isEmpty) {
      return;
    }
    final map = <String, RestsendConversation>{
      for (final item in _conversations) item.topicId: item,
    };
    for (final conversation in updates) {
      map[conversation.topicId] = conversation;
    }
    _conversations = map.values.toList()
      ..sort((a, b) => b.updatedAt.compareTo(a.updatedAt));
    notifyListeners();
  }

  @override
  void dispose() {
    _eventSub?.cancel();
    super.dispose();
  }
}
