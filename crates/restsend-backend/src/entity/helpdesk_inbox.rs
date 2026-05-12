use sea_orm::entity::prelude::*;
use sea_orm::ActiveValue::Set;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "helpdesk_inboxes")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub r#type: String,
    pub widget_config_json: String,
    pub greeting: String,
    pub greeting_enabled: bool,
    pub routing_strategy: String,
    pub offline_email: String,
    pub offline_webhook_url: String,
    pub offline_webhook_secret: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::helpdesk_inbox_member::Entity")]
    Members,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::helpdesk_inbox_member::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Members.def()
    }
}
