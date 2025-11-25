import 'package:meta/meta.dart';

import 'bridge_generated.dart/api.dart' as bridge;

/// Authentication payload passed to the Rust layer.
@immutable
class RestsendAuthInfo {
  const RestsendAuthInfo({
    required this.endpoint,
    required this.userId,
    required this.token,
    this.name,
    this.avatar,
    this.isStaff = false,
    this.isCrossDomain = false,
  });

  final String endpoint;
  final String userId;
  final String token;
  final String? name;
  final String? avatar;
  final bool isStaff;
  final bool isCrossDomain;

  factory RestsendAuthInfo.fromBridge(bridge.DartAuthInfo info) {
    return RestsendAuthInfo(
      endpoint: info.endpoint,
      userId: info.userId,
      token: info.token,
      name: info.name,
      avatar: info.avatar,
      isStaff: info.isStaff,
      isCrossDomain: info.isCrossDomain,
    );
  }

  bridge.DartAuthInfo toBridge() => bridge.DartAuthInfo(
        endpoint: endpoint,
        userId: userId,
        token: token,
        name: name,
        avatar: avatar,
        isStaff: isStaff,
        isCrossDomain: isCrossDomain,
      );
}

/// Optional client configuration.
@immutable
class RestsendClientOptions {
  const RestsendClientOptions({
    this.rootPath,
    this.dbName,
  });

  final String? rootPath;
  final String? dbName;

  bridge.ClientOptions toBridge() => bridge.ClientOptions(
        rootPath: rootPath,
        dbName: dbName,
      );
}

/// High-level typedefs to keep the public API stable.
typedef RestsendClientEvent = bridge.ClientEvent;

typedef RestsendSdkError = bridge.RestsendDartError;

typedef RestsendConversation = bridge.ConversationData;
typedef RestsendChatLog = bridge.ChatLogData;
typedef RestsendAttachment = bridge.AttachmentData;
typedef RestsendContent = bridge.ContentData;
typedef RestsendTag = bridge.TagData;
typedef RestsendChatLogsPage = bridge.ChatLogsPage;
typedef RestsendConversationListPage = bridge.ConversationListPage;
typedef RestsendSyncConversationsOptions = bridge.SyncConversationsOptions;
typedef RestsendChatRequestStatus = bridge.ChatRequestStatusData;
typedef RestsendExtra = Map<String, String>;
