use sea_orm::entity::prelude::*;
use sea_orm::ActiveValue::Set;

use crate::entity::{decode_json, encode_json};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "topics")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    pub icon: String,
    pub kind: String,
    pub owner_id: String,
    pub attendee_id: String,
    pub members: i32,
    pub last_seq: i64,
    pub multiple: bool,
    pub source: String,
    pub private: bool,
    pub knock_need_verify: bool,
    pub admins_json: String,
    pub webhooks_json: String,
    pub notice_json: String,
    pub extra_json: String,
    pub silent_white_list_json: String,
    pub silent: bool,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for crate::Topic {
    fn from(model: Model) -> Self {
        let notice = decode_json::<crate::TopicNotice>(&model.notice_json);
        crate::Topic {
            id: model.id,
            name: model.name,
            icon: model.icon,
            kind: model.kind,
            remark: String::new(),
            owner_id: model.owner_id,
            attendee_id: model.attendee_id,
            admins: decode_json(&model.admins_json),
            members: model.members as u32,
            last_seq: model.last_seq,
            multiple: model.multiple,
            source: model.source,
            private: model.private,
            knock_need_verify: model.knock_need_verify,
            webhooks: decode_json(&model.webhooks_json),
            notice: if notice.text.is_empty() {
                None
            } else {
                Some(notice)
            },
            extra: Some(decode_json(&model.extra_json)),
            silent_white_list: decode_json(&model.silent_white_list_json),
            silent: model.silent,
            enabled: model.enabled,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}

impl From<&Model> for crate::Topic {
    fn from(model: &Model) -> Self {
        let notice = decode_json::<crate::TopicNotice>(&model.notice_json);
        crate::Topic {
            id: model.id.clone(),
            name: model.name.clone(),
            icon: model.icon.clone(),
            kind: model.kind.clone(),
            remark: String::new(),
            owner_id: model.owner_id.clone(),
            attendee_id: model.attendee_id.clone(),
            admins: decode_json(&model.admins_json),
            members: model.members as u32,
            last_seq: model.last_seq,
            multiple: model.multiple,
            source: model.source.clone(),
            private: model.private,
            knock_need_verify: model.knock_need_verify,
            webhooks: decode_json(&model.webhooks_json),
            notice: if notice.text.is_empty() {
                None
            } else {
                Some(notice)
            },
            extra: Some(decode_json(&model.extra_json)),
            silent_white_list: decode_json(&model.silent_white_list_json),
            silent: model.silent,
            enabled: model.enabled,
            created_at: model.created_at.clone(),
            updated_at: model.updated_at.clone(),
        }
    }
}

impl From<(crate::Topic, &str)> for ActiveModel {
    fn from((value, now): (crate::Topic, &str)) -> Self {
        let created_at = if value.created_at.is_empty() {
            now.to_string()
        } else {
            value.created_at
        };
        ActiveModel {
            id: Set(value.id),
            name: Set(value.name),
            icon: Set(value.icon),
            kind: Set(value.kind),
            owner_id: Set(value.owner_id),
            attendee_id: Set(value.attendee_id),
            members: Set(value.members as i32),
            last_seq: Set(value.last_seq),
            multiple: Set(value.multiple),
            source: Set(value.source),
            private: Set(value.private),
            knock_need_verify: Set(value.knock_need_verify),
            admins_json: Set(encode_json(&value.admins)),
            webhooks_json: Set(encode_json(&value.webhooks)),
            notice_json: Set(encode_json(&value.notice.unwrap_or_default())),
            extra_json: Set(encode_json(&value.extra.unwrap_or_default())),
            silent_white_list_json: Set(encode_json(&value.silent_white_list)),
            silent: Set(value.silent),
            enabled: Set(value.enabled),
            created_at: Set(created_at),
            updated_at: Set(now.to_string()),
        }
    }
}

impl From<(&crate::Topic, &str)> for ActiveModel {
    fn from((value, now): (&crate::Topic, &str)) -> Self {
        ActiveModel {
            id: Set(value.id.clone()),
            name: Set(value.name.clone()),
            icon: Set(value.icon.clone()),
            kind: Set(value.kind.clone()),
            owner_id: Set(value.owner_id.clone()),
            attendee_id: Set(value.attendee_id.clone()),
            members: Set(value.members as i32),
            last_seq: Set(value.last_seq),
            multiple: Set(value.multiple),
            source: Set(value.source.clone()),
            private: Set(value.private),
            knock_need_verify: Set(value.knock_need_verify),
            admins_json: Set(encode_json(&value.admins)),
            webhooks_json: Set(encode_json(&value.webhooks)),
            notice_json: Set(encode_json(&value.notice.clone().unwrap_or_default())),
            extra_json: Set(encode_json(&value.extra.clone().unwrap_or_default())),
            silent_white_list_json: Set(encode_json(&value.silent_white_list)),
            silent: Set(value.silent),
            enabled: Set(value.enabled),
            created_at: Set(if value.created_at.is_empty() {
                now.to_string()
            } else {
                value.created_at.clone()
            }),
            updated_at: Set(now.to_string()),
        }
    }
}
