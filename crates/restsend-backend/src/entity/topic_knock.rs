use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "topic_knocks")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub topic_id: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: String,
    pub created_at: String,
    pub updated_at: String,
    pub message: String,
    pub source: String,
    pub status: String,
    pub admin_id: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
