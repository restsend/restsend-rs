use sea_orm::entity::prelude::*;
use sea_orm::ActiveValue::Set;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: String,
    pub password: String,
    pub display_name: String,
    pub avatar: String,
    pub source: String,
    pub locale: String,
    pub city: String,
    pub country: String,
    pub gender: String,
    pub public_key: String,
    pub is_staff: bool,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for crate::User {
    fn from(model: Model) -> Self {
        crate::User {
            user_id: model.user_id,
            name: model.display_name,
            avatar: model.avatar,
            public_key: model.public_key,
            locale: model.locale,
            city: model.city,
            country: model.country,
            source: model.source,
            gender: model.gender,
            is_staff: model.is_staff,
            enabled: model.enabled,
            created_at: model.created_at,
            ..crate::User::default()
        }
    }
}

impl From<&Model> for crate::User {
    fn from(model: &Model) -> Self {
        crate::User {
            user_id: model.user_id.clone(),
            name: model.display_name.clone(),
            avatar: model.avatar.clone(),
            public_key: model.public_key.clone(),
            locale: model.locale.clone(),
            city: model.city.clone(),
            country: model.country.clone(),
            source: model.source.clone(),
            gender: model.gender.clone(),
            is_staff: model.is_staff,
            enabled: model.enabled,
            created_at: model.created_at.clone(),
            ..crate::User::default()
        }
    }
}

impl From<(crate::User, &str)> for ActiveModel {
    fn from((value, now): (crate::User, &str)) -> Self {
        let created_at = if value.created_at.is_empty() {
            now.to_string()
        } else {
            value.created_at
        };
        ActiveModel {
            user_id: Set(value.user_id),
            password: Set(String::new()),
            display_name: Set(value.name),
            avatar: Set(value.avatar),
            source: Set(value.source),
            locale: Set(value.locale),
            city: Set(value.city),
            country: Set(value.country),
            gender: Set(value.gender),
            public_key: Set(value.public_key),
            is_staff: Set(value.is_staff),
            enabled: Set(value.enabled),
            created_at: Set(created_at),
            updated_at: Set(now.to_string()),
        }
    }
}

impl From<(&crate::User, &str)> for ActiveModel {
    fn from((value, now): (&crate::User, &str)) -> Self {
        ActiveModel {
            user_id: Set(value.user_id.clone()),
            password: Set(String::new()),
            display_name: Set(value.name.clone()),
            avatar: Set(value.avatar.clone()),
            source: Set(value.source.clone()),
            locale: Set(value.locale.clone()),
            city: Set(value.city.clone()),
            country: Set(value.country.clone()),
            gender: Set(value.gender.clone()),
            public_key: Set(value.public_key.clone()),
            is_staff: Set(value.is_staff),
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
