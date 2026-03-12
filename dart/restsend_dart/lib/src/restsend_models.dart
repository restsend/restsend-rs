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

/// Default (empty) [bridge.ContentData] — mirrors Rust's `Content::default()`.
bridge.ContentData _emptyContent({String contentType = ''}) => bridge.ContentData(
      contentType: contentType,
      encrypted: false,
      checksum: 0,
      text: '',
      placeholder: '',
      thumbnail: '',
      duration: '',
      size: BigInt.zero,
      width: 0,
      height: 0,
      mentions: const [],
      mentionAll: false,
      reply: '',
      replyContent: null,
      attachment: null,
      extra: null,
      unreadable: false,
    );

/// Convenience factory methods for [RestsendContent] / [bridge.ContentData].
///
/// Mirrors the Rust helpers:
///   `Content::new_text(ContentType::Text, "hello")`
///   `Content::new(ContentType::Image)`  etc.
extension RestsendContentFactory on Never {
  // This extension only exists to document the static helpers below.
  // Use [RestsendContentX] factories directly.
}

/// Static factory helpers — call like `RestsendContentX.text("hello")`.
abstract final class RestsendContentX {
  /// Plain text message.
  static bridge.ContentData text(
    String text, {
    List<String> mentions = const [],
    bool mentionAll = false,
    String reply = '',
    String? replyContent,
    Map<String, String>? extra,
  }) =>
      _emptyContent(contentType: 'text').copyWith(
        text: text,
        mentions: mentions,
        mentionAll: mentionAll,
        reply: reply,
        replyContent: replyContent,
        extra: extra,
      );

  /// Image message.
  static bridge.ContentData image({
    required String url,
    String placeholder = '',
    String thumbnail = '',
    double width = 0,
    double height = 0,
    BigInt? size,
    Map<String, String>? extra,
  }) =>
      _emptyContent(contentType: 'image').copyWith(
        text: url,
        placeholder: placeholder,
        thumbnail: thumbnail,
        width: width,
        height: height,
        size: size ?? BigInt.zero,
        extra: extra,
      );

  /// Video message.
  static bridge.ContentData video({
    required String url,
    String thumbnail = '',
    String duration = '',
    double width = 0,
    double height = 0,
    BigInt? size,
    Map<String, String>? extra,
  }) =>
      _emptyContent(contentType: 'video').copyWith(
        text: url,
        thumbnail: thumbnail,
        duration: duration,
        width: width,
        height: height,
        size: size ?? BigInt.zero,
        extra: extra,
      );

  /// Voice / audio message.
  static bridge.ContentData voice({
    required String url,
    String duration = '',
    BigInt? size,
    Map<String, String>? extra,
  }) =>
      _emptyContent(contentType: 'voice').copyWith(
        text: url,
        duration: duration,
        size: size ?? BigInt.zero,
        extra: extra,
      );

  /// File message.
  static bridge.ContentData file({
    required String url,
    String placeholder = '',
    BigInt? size,
    Map<String, String>? extra,
  }) =>
      _emptyContent(contentType: 'file').copyWith(
        text: url,
        placeholder: placeholder,
        size: size ?? BigInt.zero,
        extra: extra,
      );

  /// Sticker / emoji message.
  static bridge.ContentData sticker(
    String url, {
    Map<String, String>? extra,
  }) =>
      _emptyContent(contentType: 'sticker').copyWith(
        text: url,
        extra: extra,
      );

  /// Link / URL message.
  static bridge.ContentData link(
    String url, {
    String placeholder = '',
    Map<String, String>? extra,
  }) =>
      _emptyContent(contentType: 'link').copyWith(
        text: url,
        placeholder: placeholder,
        extra: extra,
      );

  /// Location message (`text` encodes coords, `placeholder` is the label).
  static bridge.ContentData location({
    required String coordinates,
    String label = '',
    Map<String, String>? extra,
  }) =>
      _emptyContent(contentType: 'location').copyWith(
        text: coordinates,
        placeholder: label,
        extra: extra,
      );

  /// Custom / arbitrary content type.
  static bridge.ContentData custom({
    required String contentType,
    String text = '',
    Map<String, String>? extra,
  }) =>
      _emptyContent(contentType: contentType).copyWith(
        text: text,
        extra: extra,
      );
}

/// [copyWith] helper for the auto-generated [bridge.ContentData].
extension ContentDataCopyWith on bridge.ContentData {
  bridge.ContentData copyWith({
    String? contentType,
    bool? encrypted,
    int? checksum,
    String? text,
    String? placeholder,
    String? thumbnail,
    String? duration,
    BigInt? size,
    double? width,
    double? height,
    List<String>? mentions,
    bool? mentionAll,
    String? reply,
    String? replyContent,
    bridge.AttachmentData? attachment,
    Map<String, String>? extra,
    bool? unreadable,
  }) =>
      bridge.ContentData(
        contentType: contentType ?? this.contentType,
        encrypted: encrypted ?? this.encrypted,
        checksum: checksum ?? this.checksum,
        text: text ?? this.text,
        placeholder: placeholder ?? this.placeholder,
        thumbnail: thumbnail ?? this.thumbnail,
        duration: duration ?? this.duration,
        size: size ?? this.size,
        width: width ?? this.width,
        height: height ?? this.height,
        mentions: mentions ?? this.mentions,
        mentionAll: mentionAll ?? this.mentionAll,
        reply: reply ?? this.reply,
        replyContent: replyContent ?? this.replyContent,
        attachment: attachment ?? this.attachment,
        extra: extra ?? this.extra,
        unreadable: unreadable ?? this.unreadable,
      );
}
