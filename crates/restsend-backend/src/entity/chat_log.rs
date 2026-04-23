use sea_orm::entity::prelude::*;
use sea_orm::ActiveValue::Set;

use crate::entity::{decode_json, encode_json};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "chat_logs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub topic_id: String,
    pub seq: i64,
    pub sender_id: String,
    pub content_json: String,
    pub deleted_by_json: String,
    pub read: bool,
    pub recall: bool,
    pub source: String,
    pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for crate::ChatLog {
    fn from(model: Model) -> Self {
        crate::ChatLog {
            id: model.id,
            topic_id: model.topic_id,
            seq: model.seq,
            sender_id: model.sender_id,
            content: decode_json(&model.content_json),
            created_at: model.created_at,
            read: model.read,
            recall: model.recall,
            deleted_by: decode_json(&model.deleted_by_json),
        }
    }
}

impl From<&Model> for crate::ChatLog {
    fn from(model: &Model) -> Self {
        crate::ChatLog {
            id: model.id.clone(),
            topic_id: model.topic_id.clone(),
            seq: model.seq,
            sender_id: model.sender_id.clone(),
            content: decode_json(&model.content_json),
            created_at: model.created_at.clone(),
            read: model.read,
            recall: model.recall,
            deleted_by: decode_json(&model.deleted_by_json),
        }
    }
}

impl From<crate::ChatLog> for ActiveModel {
    fn from(value: crate::ChatLog) -> Self {
        ActiveModel {
            id: Set(value.id),
            topic_id: Set(value.topic_id),
            seq: Set(value.seq),
            sender_id: Set(value.sender_id),
            content_json: Set(encode_json(&value.content)),
            deleted_by_json: Set(encode_json(&value.deleted_by)),
            read: Set(value.read),
            recall: Set(value.recall),
            source: Set(value
                .content
                .extra
                .as_ref()
                .and_then(|m| m.get("source").cloned())
                .unwrap_or_default()),
            created_at: Set(value.created_at),
        }
    }
}

impl From<&crate::ChatLog> for ActiveModel {
    fn from(value: &crate::ChatLog) -> Self {
        ActiveModel {
            id: Set(value.id.clone()),
            topic_id: Set(value.topic_id.clone()),
            seq: Set(value.seq),
            sender_id: Set(value.sender_id.clone()),
            content_json: Set(encode_json(&value.content)),
            deleted_by_json: Set(encode_json(&value.deleted_by)),
            read: Set(value.read),
            recall: Set(value.recall),
            source: Set(value
                .content
                .extra
                .as_ref()
                .and_then(|m| m.get("source").cloned())
                .unwrap_or_default()),
            created_at: Set(value.created_at.clone()),
        }
    }
}
