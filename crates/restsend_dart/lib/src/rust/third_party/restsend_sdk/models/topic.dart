// This file is automatically generated, so please do not edit it.
// @generated by `flutter_rust_bridge`@ 2.6.0.

// ignore_for_file: invalid_use_of_internal_member, unused_import, unnecessary_import

import '../../../frb_generated.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';

// These function are ignored because they are on traits that is not defined in current crate (put an empty `#[frb]` on it to unignore): `fmt`, `fmt`, `lower_return`, `lower_return`, `lower`, `lower`, `lower`, `lower`, `try_convert_unexpected_callback_error`, `try_convert_unexpected_callback_error`, `try_lift_successful_return`, `try_lift_successful_return`, `try_lift`, `try_lift`, `try_lift`, `try_lift`, `try_read`, `try_read`, `try_read`, `try_read`, `write`, `write`, `write`, `write`

class Topic {
  final String id;
  final String name;
  final String icon;
  final String remark;
  final String ownerId;
  final String attendeeId;
  final List<String> admins;
  final int members;
  final PlatformInt64 lastSeq;
  final bool multiple;
  final String source;
  final bool private;
  final String createdAt;
  final String updatedAt;
  final TopicNotice? notice;
  final Map<String, String>? extra;
  final bool silent;
  final PlatformInt64 cachedAt;

  const Topic({
    required this.id,
    required this.name,
    required this.icon,
    required this.remark,
    required this.ownerId,
    required this.attendeeId,
    required this.admins,
    required this.members,
    required this.lastSeq,
    required this.multiple,
    required this.source,
    required this.private,
    required this.createdAt,
    required this.updatedAt,
    this.notice,
    this.extra,
    required this.silent,
    required this.cachedAt,
  });

  static Future<Topic> default_() =>
      RustLib.instance.api.restsendSdkModelsTopicTopicDefault();

  // HINT: Make it `#[frb(sync)]` to let it become the default constructor of Dart class.
  static Future<Topic> newInstance({required String topicId}) =>
      RustLib.instance.api.restsendSdkModelsTopicTopicNew(topicId: topicId);

  @override
  int get hashCode =>
      id.hashCode ^
      name.hashCode ^
      icon.hashCode ^
      remark.hashCode ^
      ownerId.hashCode ^
      attendeeId.hashCode ^
      admins.hashCode ^
      members.hashCode ^
      lastSeq.hashCode ^
      multiple.hashCode ^
      source.hashCode ^
      private.hashCode ^
      createdAt.hashCode ^
      updatedAt.hashCode ^
      notice.hashCode ^
      extra.hashCode ^
      silent.hashCode ^
      cachedAt.hashCode;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is Topic &&
          runtimeType == other.runtimeType &&
          id == other.id &&
          name == other.name &&
          icon == other.icon &&
          remark == other.remark &&
          ownerId == other.ownerId &&
          attendeeId == other.attendeeId &&
          admins == other.admins &&
          members == other.members &&
          lastSeq == other.lastSeq &&
          multiple == other.multiple &&
          source == other.source &&
          private == other.private &&
          createdAt == other.createdAt &&
          updatedAt == other.updatedAt &&
          notice == other.notice &&
          extra == other.extra &&
          silent == other.silent &&
          cachedAt == other.cachedAt;
}

class TopicNotice {
  final String text;
  final String publisher;
  final String updatedAt;

  const TopicNotice({
    required this.text,
    required this.publisher,
    required this.updatedAt,
  });

  // HINT: Make it `#[frb(sync)]` to let it become the default constructor of Dart class.
  static Future<TopicNotice> newInstance(
          {required String text,
          required String publisher,
          required String updatedAt}) =>
      RustLib.instance.api.restsendSdkModelsTopicTopicNoticeNew(
          text: text, publisher: publisher, updatedAt: updatedAt);

  @override
  int get hashCode => text.hashCode ^ publisher.hashCode ^ updatedAt.hashCode;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is TopicNotice &&
          runtimeType == other.runtimeType &&
          text == other.text &&
          publisher == other.publisher &&
          updatedAt == other.updatedAt;
}
