use std::sync::Arc;

use sea_orm::DatabaseConnection;

use crate::app::AppConfig;
use crate::infra::event::EventBus;
use crate::infra::metrics::RuntimeMetrics;
use crate::infra::presence::PresenceHub;
use crate::infra::task_pool::TaskPool;
use crate::infra::webhook::WebhookSender;
use crate::infra::websocket::WsHub;
use crate::services::{
    AuthService, ChatService, ConversationService, RelationService, TopicService, UserService,
};

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub db: DatabaseConnection,
    pub ws_hub: Arc<WsHub>,
    pub presence_hub: Arc<PresenceHub>,
    pub message_pool: Arc<TaskPool>,
    pub push_pool: Arc<TaskPool>,
    pub webhook_pool: Arc<TaskPool>,
    pub event_bus: Arc<EventBus>,
    pub metrics: Arc<RuntimeMetrics>,
    pub webhook_sender: Arc<WebhookSender>,
    pub cluster_push_client: reqwest::Client,
    pub webhook_targets: Arc<Vec<String>>,
    pub user_service: Arc<UserService>,
    pub auth_service: Arc<AuthService>,
    pub relation_service: Arc<RelationService>,
    pub topic_service: Arc<TopicService>,
    pub conversation_service: Arc<ConversationService>,
    pub chat_service: Arc<ChatService>,
}
