use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "attachments")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub path: String,
    pub file_name: String,
    pub store_path: String,
    pub owner_id: String,
    pub topic_id: String,
    pub size: i64,
    pub ext: String,
    pub private: bool,
    pub external: bool,
    pub tags: String,
    pub remark: String,
    pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
