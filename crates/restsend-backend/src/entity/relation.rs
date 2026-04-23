use sea_orm::entity::prelude::*;
use sea_orm::ActiveValue::Set;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "relations")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub owner_id: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub target_id: String,
    pub is_contact: bool,
    pub is_star: bool,
    pub is_blocked: bool,
    pub remark: String,
    pub source: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for crate::Relation {
    fn from(model: Model) -> Self {
        crate::Relation {
            owner_id: model.owner_id,
            target_id: model.target_id,
            is_contact: model.is_contact,
            is_star: model.is_star,
            is_blocked: model.is_blocked,
            remark: model.remark,
            source: model.source,
        }
    }
}

impl From<&Model> for crate::Relation {
    fn from(model: &Model) -> Self {
        crate::Relation {
            owner_id: model.owner_id.clone(),
            target_id: model.target_id.clone(),
            is_contact: model.is_contact,
            is_star: model.is_star,
            is_blocked: model.is_blocked,
            remark: model.remark.clone(),
            source: model.source.clone(),
        }
    }
}

impl From<(crate::Relation, &str)> for ActiveModel {
    fn from((value, now): (crate::Relation, &str)) -> Self {
        ActiveModel {
            owner_id: Set(value.owner_id),
            target_id: Set(value.target_id),
            is_contact: Set(value.is_contact),
            is_star: Set(value.is_star),
            is_blocked: Set(value.is_blocked),
            remark: Set(value.remark),
            source: Set(value.source),
            updated_at: Set(now.to_string()),
        }
    }
}

impl From<(&crate::Relation, &str)> for ActiveModel {
    fn from((value, now): (&crate::Relation, &str)) -> Self {
        ActiveModel {
            owner_id: Set(value.owner_id.clone()),
            target_id: Set(value.target_id.clone()),
            is_contact: Set(value.is_contact),
            is_star: Set(value.is_star),
            is_blocked: Set(value.is_blocked),
            remark: Set(value.remark.clone()),
            source: Set(value.source.clone()),
            updated_at: Set(now.to_string()),
        }
    }
}
