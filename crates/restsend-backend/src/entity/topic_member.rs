use sea_orm::entity::prelude::*;
use sea_orm::ActiveValue::Set;

use crate::entity::{decode_json, encode_json};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "topic_members")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub topic_id: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: String,
    pub name: String,
    pub source: String,
    pub role: String,
    pub silence_at: Option<String>,
    pub joined_at: String,
    pub updated_at: String,
    pub extra_json: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for crate::TopicMember {
    fn from(model: Model) -> Self {
        crate::TopicMember {
            topic_id: model.topic_id,
            user_id: model.user_id,
            name: model.name,
            source: model.source,
            role: model.role,
            silence_at: model.silence_at,
            joined_at: model.joined_at,
            updated_at: model.updated_at,
            extra: Some(decode_json(&model.extra_json)),
        }
    }
}

impl From<&Model> for crate::TopicMember {
    fn from(model: &Model) -> Self {
        crate::TopicMember {
            topic_id: model.topic_id.clone(),
            user_id: model.user_id.clone(),
            name: model.name.clone(),
            source: model.source.clone(),
            role: model.role.clone(),
            silence_at: model.silence_at.clone(),
            joined_at: model.joined_at.clone(),
            updated_at: model.updated_at.clone(),
            extra: Some(decode_json(&model.extra_json)),
        }
    }
}

impl From<(crate::TopicMember, &str)> for ActiveModel {
    fn from((value, now): (crate::TopicMember, &str)) -> Self {
        let role = if value.role.is_empty() {
            "member".to_string()
        } else {
            value.role
        };
        let joined_at = if value.joined_at.is_empty() {
            now.to_string()
        } else {
            value.joined_at
        };
        ActiveModel {
            topic_id: Set(value.topic_id),
            user_id: Set(value.user_id),
            name: Set(value.name),
            source: Set(value.source),
            role: Set(role),
            silence_at: Set(value.silence_at),
            joined_at: Set(joined_at),
            updated_at: Set(now.to_string()),
            extra_json: Set(encode_json(&value.extra.unwrap_or_default())),
        }
    }
}

impl From<(&crate::TopicMember, &str)> for ActiveModel {
    fn from((value, now): (&crate::TopicMember, &str)) -> Self {
        ActiveModel {
            topic_id: Set(value.topic_id.clone()),
            user_id: Set(value.user_id.clone()),
            name: Set(value.name.clone()),
            source: Set(value.source.clone()),
            role: Set(if value.role.is_empty() {
                "member".to_string()
            } else {
                value.role.clone()
            }),
            silence_at: Set(value.silence_at.clone()),
            joined_at: Set(if value.joined_at.is_empty() {
                now.to_string()
            } else {
                value.joined_at.clone()
            }),
            updated_at: Set(now.to_string()),
            extra_json: Set(encode_json(&value.extra.clone().unwrap_or_default())),
        }
    }
}
