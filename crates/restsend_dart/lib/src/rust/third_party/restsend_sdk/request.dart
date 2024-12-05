// This file is automatically generated, so please do not edit it.
// @generated by `flutter_rust_bridge`@ 2.6.0.

// ignore_for_file: invalid_use_of_internal_member, unused_import, unnecessary_import

import '../../frb_generated.dart';
import 'models/chat_log.dart';
import 'models/user.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';

class ChatRequest {
  final String reqType;
  final String chatId;
  final int code;
  final String topicId;
  final PlatformInt64 seq;
  final String attendee;
  final User? attendeeProfile;
  final String createdAt;
  final Content? content;
  final String? e2EContent;
  final String? message;
  final String? source;

  const ChatRequest({
    required this.reqType,
    required this.chatId,
    required this.code,
    required this.topicId,
    required this.seq,
    required this.attendee,
    this.attendeeProfile,
    required this.createdAt,
    this.content,
    this.e2EContent,
    this.message,
    this.source,
  });

  @override
  int get hashCode =>
      reqType.hashCode ^
      chatId.hashCode ^
      code.hashCode ^
      topicId.hashCode ^
      seq.hashCode ^
      attendee.hashCode ^
      attendeeProfile.hashCode ^
      createdAt.hashCode ^
      content.hashCode ^
      e2EContent.hashCode ^
      message.hashCode ^
      source.hashCode;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ChatRequest &&
          runtimeType == other.runtimeType &&
          reqType == other.reqType &&
          chatId == other.chatId &&
          code == other.code &&
          topicId == other.topicId &&
          seq == other.seq &&
          attendee == other.attendee &&
          attendeeProfile == other.attendeeProfile &&
          createdAt == other.createdAt &&
          content == other.content &&
          e2EContent == other.e2EContent &&
          message == other.message &&
          source == other.source;
}