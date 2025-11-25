import 'dart:async';

import 'bridge_generated.dart/api.dart' as bridge;
import 'restsend_models.dart';
import 'runtime.dart';

/// Thin wrapper around the generated bridge functions.
class RestsendClient {
  RestsendClient._(this._handle);

  final bridge.ClientHandle _handle;

  /// Creates a new client with the given auth payload.
  static Future<RestsendClient> create({
    required RestsendAuthInfo auth,
    RestsendClientOptions? options,
  }) async {
    await RestsendRuntime.ensureInitialized();
    final handle = await bridge.createClient(
      auth: auth.toBridge(),
      options: options?.toBridge(),
    );
    return RestsendClient._(handle);
  }

  bridge.ClientHandle get handle => _handle;

  Future<void> connect() => bridge.connectClient(client: _handle);

  Future<void> shutdown() => bridge.shutdownClient(client: _handle);

  Future<String> connectionStatus() =>
      bridge.getConnectionStatus(client: _handle);

  Future<int> lastAliveAt() async {
    final value = await bridge.getLastAliveAt(client: _handle);
    return value.toInt();
  }

  Future<void> markAppActive() => bridge.appActive(client: _handle);

  Future<void> setKeepaliveInterval(int seconds) =>
      bridge.setKeepaliveInterval(client: _handle, secs: seconds);

  Future<void> setPingInterval(int seconds) =>
      bridge.setPingInterval(client: _handle, secs: seconds);

  Future<void> setMaxRetry(int count) =>
      bridge.setMaxRetry(client: _handle, count: count);

  Future<int> unreadCount() => bridge.getUnreadCount(client: _handle);

  Stream<RestsendClientEvent> events() =>
      bridge.listenClientEvents(client: _handle);

  Future<RestsendConversation> createChat(String userId) =>
      bridge.createChat(client: _handle, userId: userId);

  Future<void> cleanMessages(String topicId) =>
      bridge.cleanMessages(client: _handle, topicId: topicId);

  Future<void> removeMessages(
    String topicId,
    List<String> chatIds, {
    bool syncToServer = true,
  }) =>
      bridge.removeMessages(
        client: _handle,
        topicId: topicId,
        chatIds: chatIds,
        syncToServer: syncToServer,
      );

  Future<RestsendChatLog?> getChatLog(String topicId, String chatId) =>
      bridge.getChatLog(client: _handle, topicId: topicId, chatId: chatId);

  Future<void> saveChatLogs(List<RestsendChatLog> logs) =>
      bridge.saveChatLogs(client: _handle, logs: logs);

  Future<RestsendConversation?> getConversation(String topicId) =>
      bridge.getConversation(client: _handle, topicId: topicId);

  Future<RestsendConversationListPage> listConversations({
    String? updatedAt,
    int limit = 50,
  }) =>
      bridge.listConversations(
        client: _handle,
        updatedAt: updatedAt,
        limit: limit,
      );

  Future<void> syncConversations({
    RestsendSyncConversationsOptions? options,
  }) =>
      bridge.syncConversations(
        client: _handle,
        options: options,
      );

  Future<void> removeConversation(String topicId) =>
      bridge.removeConversation(client: _handle, topicId: topicId);

  Future<RestsendConversation> setConversationRemark(
    String topicId,
    String? remark,
  ) =>
      bridge.setConversationRemark(
        client: _handle,
        topicId: topicId,
        remark: remark,
      );

  Future<RestsendConversation> setConversationSticky(
    String topicId,
    bool sticky,
  ) =>
      bridge.setConversationSticky(
        client: _handle,
        topicId: topicId,
        sticky: sticky,
      );

  Future<RestsendConversation> setConversationMute(
    String topicId,
    bool mute,
  ) =>
      bridge.setConversationMute(
        client: _handle,
        topicId: topicId,
        mute: mute,
      );

  Future<void> setConversationRead(
    String topicId, {
    bool heavy = false,
  }) =>
      bridge.setConversationRead(
        client: _handle,
        topicId: topicId,
        heavy: heavy,
      );

  Future<void> setAllConversationsRead() =>
      bridge.setAllConversationsRead(client: _handle);

  Future<RestsendConversation> setConversationTags(
    String topicId,
    List<RestsendTag>? tags,
  ) =>
      bridge.setConversationTags(
        client: _handle,
        topicId: topicId,
        tags: tags,
      );

  Future<RestsendConversation> setConversationExtra(
    String topicId,
    RestsendExtra? extra,
  ) =>
      bridge.setConversationExtra(
        client: _handle,
        topicId: topicId,
        extra: extra,
      );

  Future<void> clearConversation(String topicId) =>
      bridge.clearConversation(client: _handle, topicId: topicId);

  Future<RestsendChatLogsPage> getChatLogsLocal({
    required String topicId,
    int startSeq = 0,
    int? endSeq,
    int limit = 50,
  }) =>
      bridge.getChatLogsLocal(
        client: _handle,
        topicId: topicId,
        startSeq: startSeq,
        endSeq: endSeq,
        limit: limit,
      );

  Future<RestsendChatLogsPage> syncChatLogs({
    required String topicId,
    int? lastSeq,
    int limit = 50,
    bool heavy = false,
    bool? ensureConversationLastVersion,
  }) =>
      bridge.syncChatLogs(
        client: _handle,
        topicId: topicId,
        lastSeq: lastSeq,
        limit: limit,
        heavy: heavy,
        ensureConversationLastVersion: ensureConversationLastVersion,
      );

  Future<String> sendTextMessage(
    String topicId,
    String text, {
    List<String>? mentions,
    String? replyTo,
  }) =>
      bridge.sendTextMessage(
        client: _handle,
        topicId: topicId,
        text: text,
        mentions: mentions,
        replyTo: replyTo,
      );

  Future<String> sendCustomMessage(
    String topicId,
    RestsendContent content,
  ) =>
      bridge.sendCustomMessage(
        client: _handle,
        topicId: topicId,
        content: content,
      );

  Future<String> recallMessage(String topicId, String chatId) =>
      bridge.recallMessage(
        client: _handle,
        topicId: topicId,
        chatId: chatId,
      );

  Future<void> sendTyping(String topicId) =>
      bridge.sendTyping(client: _handle, topicId: topicId);

  Future<void> sendReadReceipt(String topicId) =>
      bridge.sendReadReceipt(client: _handle, topicId: topicId);

  Future<String> updateMessageExtra(
    String topicId,
    String chatId,
    RestsendExtra? extra,
  ) =>
      bridge.updateMessageExtra(
        client: _handle,
        topicId: topicId,
        chatId: chatId,
        extra: extra,
      );

  Future<String> sendPing(String content) =>
      bridge.sendPing(client: _handle, content: content);
}
