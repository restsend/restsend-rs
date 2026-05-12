use sea_orm::entity::prelude::*;
use sea_orm::ActiveValue::Set;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "helpdesk_inbox_members")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub inbox_id: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: String,
    pub role: String,
    pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::helpdesk_inbox::Entity",
        from = "Column::InboxId",
        to = "super::helpdesk_inbox::Column::Id"
    )]
    Inbox,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::helpdesk_inbox::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Inbox.def()
    }
}
