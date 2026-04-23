use sea_orm::entity::prelude::*;
use sea_orm::ActiveValue::Set;

use crate::entity::{decode_json, encode_json};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "conversations")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub owner_id: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub topic_id: String,
    pub updated_at: String,
    pub sticky: bool,
    pub mute: bool,
    pub remark: Option<String>,
    pub unread: i64,
    pub start_seq: i64,
    pub last_seq: i64,
    pub last_read_seq: i64,
    pub last_read_at: Option<String>,
    pub multiple: bool,
    pub attendee: String,
    pub members: i64,
    pub name: String,
    pub icon: String,
    pub kind: String,
    pub source: String,
    pub last_sender_id: String,
    pub last_message_json: String,
    pub last_message_at: String,
    pub last_message_seq: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for crate::Conversation {
    fn from(model: Model) -> Self {
        crate::Conversation {
            owner_id: model.owner_id,
            topic_id: model.topic_id,
            updated_at: model.updated_at,
            start_seq: model.start_seq,
            sticky: model.sticky,
            mute: model.mute,
            remark: model.remark,
            unread: model.unread,
            last_seq: model.last_seq,
            last_read_seq: model.last_read_seq,
            last_read_at: model.last_read_at,
            multiple: model.multiple,
            attendee: model.attendee,
            members: model.members,
            name: model.name,
            icon: model.icon,
            kind: model.kind,
            source: model.source,
            last_sender_id: model.last_sender_id,
            last_message: if model.last_message_json == "{}" || model.last_message_json.is_empty() {
                None
            } else {
                Some(decode_json(&model.last_message_json))
            },
            last_message_at: model.last_message_at,
            last_message_seq: if model.last_message_seq == 0 {
                None
            } else {
                Some(model.last_message_seq)
            },
            extra: None,
            topic_extra: None,
            topic_owner_id: None,
            topic_created_at: None,
            tags: None,
        }
    }
}

impl From<&Model> for crate::Conversation {
    fn from(model: &Model) -> Self {
        crate::Conversation {
            owner_id: model.owner_id.clone(),
            topic_id: model.topic_id.clone(),
            updated_at: model.updated_at.clone(),
            start_seq: model.start_seq,
            sticky: model.sticky,
            mute: model.mute,
            remark: model.remark.clone(),
            unread: model.unread,
            last_seq: model.last_seq,
            last_read_seq: model.last_read_seq,
            last_read_at: model.last_read_at.clone(),
            multiple: model.multiple,
            attendee: model.attendee.clone(),
            members: model.members,
            name: model.name.clone(),
            icon: model.icon.clone(),
            kind: model.kind.clone(),
            source: model.source.clone(),
            last_sender_id: model.last_sender_id.clone(),
            last_message: if model.last_message_json == "{}" || model.last_message_json.is_empty() {
                None
            } else {
                Some(decode_json(&model.last_message_json))
            },
            last_message_at: model.last_message_at.clone(),
            last_message_seq: if model.last_message_seq == 0 {
                None
            } else {
                Some(model.last_message_seq)
            },
            extra: None,
            topic_extra: None,
            topic_owner_id: None,
            topic_created_at: None,
            tags: None,
        }
    }
}

impl From<(crate::Conversation, &str)> for ActiveModel {
    fn from((value, now): (crate::Conversation, &str)) -> Self {
        let updated_at = if value.updated_at.is_empty() {
            now.to_string()
        } else {
            value.updated_at
        };
        ActiveModel {
            owner_id: Set(value.owner_id),
            topic_id: Set(value.topic_id),
            updated_at: Set(updated_at),
            sticky: Set(value.sticky),
            mute: Set(value.mute),
            remark: Set(value.remark),
            unread: Set(value.unread),
            start_seq: Set(value.start_seq),
            last_seq: Set(value.last_seq),
            last_read_seq: Set(value.last_read_seq),
            last_read_at: Set(value.last_read_at),
            multiple: Set(value.multiple),
            attendee: Set(value.attendee),
            members: Set(value.members),
            name: Set(value.name),
            icon: Set(value.icon),
            kind: Set(value.kind),
            source: Set(value.source),
            last_sender_id: Set(value.last_sender_id),
            last_message_json: Set(value
                .last_message
                .as_ref()
                .map(encode_json)
                .unwrap_or_else(|| "{}".to_string())),
            last_message_at: Set(value.last_message_at),
            last_message_seq: Set(value.last_message_seq.unwrap_or_default()),
        }
    }
}

impl From<(&crate::Conversation, &str)> for ActiveModel {
    fn from((value, now): (&crate::Conversation, &str)) -> Self {
        ActiveModel {
            owner_id: Set(value.owner_id.clone()),
            topic_id: Set(value.topic_id.clone()),
            updated_at: Set(if value.updated_at.is_empty() {
                now.to_string()
            } else {
                value.updated_at.clone()
            }),
            sticky: Set(value.sticky),
            mute: Set(value.mute),
            remark: Set(value.remark.clone()),
            unread: Set(value.unread),
            start_seq: Set(value.start_seq),
            last_seq: Set(value.last_seq),
            last_read_seq: Set(value.last_read_seq),
            last_read_at: Set(value.last_read_at.clone()),
            multiple: Set(value.multiple),
            attendee: Set(value.attendee.clone()),
            members: Set(value.members),
            name: Set(value.name.clone()),
            icon: Set(value.icon.clone()),
            kind: Set(value.kind.clone()),
            source: Set(value.source.clone()),
            last_sender_id: Set(value.last_sender_id.clone()),
            last_message_json: Set(value
                .last_message
                .as_ref()
                .map(encode_json)
                .unwrap_or_else(|| "{}".to_string())),
            last_message_at: Set(value.last_message_at.clone()),
            last_message_seq: Set(value.last_message_seq.unwrap_or_default()),
        }
    }
}
