use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "stats_daily")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub date: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub metric: String,
    pub value: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
