use super::RsClient;
use restsend_sdk::models;
use restsend_sdk::models::ListUserResult;
use restsend_sdk::Result;
use std::sync::Arc;

#[uniffi::export]
impl RsClient {
    // Topic API
    pub async fn get_topic(self: Arc<Self>, topic_id: String) -> Option<models::Topic> {
        self.0.get_topic(&topic_id).await
    }
    pub async fn get_topic_admins(self: Arc<Self>, topic_id: String) -> Option<Vec<models::User>> {
        self.0.get_topic_admins(&topic_id).await
    }
    pub async fn get_topic_owner(self: Arc<Self>, topic_id: String) -> Option<models::User> {
        self.0.get_topic_owner(&topic_id).await
    }

    pub async fn get_topic_members(
        self: Arc<Self>,
        topic_id: String,
        updated_at: String,
        limit: u32,
    ) -> Option<ListUserResult> {
        self.0
            .get_topic_members(&topic_id, &updated_at, limit)
            .await
    }

    pub async fn get_topic_knocks(
        self: Arc<Self>,
        topic_id: String,
    ) -> Option<Vec<models::TopicKnock>> {
        self.0.get_topic_knocks(&topic_id).await
    }

    pub async fn update_topic(
        self: Arc<Self>,
        topic_id: String,
        name: Option<String>,
        icon: Option<String>,
    ) -> Result<()> {
        self.0.update_topic(&topic_id, name, icon).await
    }

    pub async fn update_topic_notice(
        self: Arc<Self>,
        topic_id: String,
        text: String,
    ) -> Result<()> {
        self.0.update_topic_notice(&topic_id, &text).await
    }

    pub async fn silent_topic(
        self: Arc<Self>,
        topic_id: String,
        duration: Option<String>,
    ) -> Result<()> {
        self.0.silent_topic(&topic_id, duration).await
    }

    pub async fn silent_topic_member(
        self: Arc<Self>,
        topic_id: String,
        user_id: String,
        duration: Option<String>,
    ) -> Result<()> {
        self.0
            .silent_topic_member(&topic_id, &user_id, duration)
            .await
    }

    pub async fn add_topic_admin(self: Arc<Self>, topic_id: String, user_id: String) -> Result<()> {
        self.0.add_topic_admin(&topic_id, &user_id).await
    }

    pub async fn remove_topic_admin(
        self: Arc<Self>,
        topic_id: String,
        user_id: String,
    ) -> Result<()> {
        self.0.remove_topic_admin(&topic_id, &user_id).await
    }

    pub async fn transfer_topic(self: Arc<Self>, topic_id: String, user_id: String) -> Result<()> {
        self.0.transfer_topic(&topic_id, &user_id).await
    }

    pub async fn quit_topic(self: Arc<Self>, topic_id: String) -> Result<()> {
        self.0.quit_topic(&topic_id).await
    }

    pub async fn dismiss_topic(self: Arc<Self>, topic_id: String) -> Result<()> {
        self.0.dismiss_topic(&topic_id).await
    }

    pub async fn accept_topic_join(
        self: Arc<Self>,
        topic_id: String,
        user_id: String,
        memo: String,
    ) -> Result<()> {
        self.0.accept_topic_join(&topic_id, &user_id, &memo).await
    }

    pub async fn decline_topic_join(
        self: Arc<Self>,
        topic_id: String,
        user_id: String,
        message: Option<String>,
    ) -> Result<()> {
        self.0
            .decline_topic_join(&topic_id, &user_id, message)
            .await
    }

    pub async fn remove_topic_member(
        self: Arc<Self>,
        topic_id: String,
        user_id: String,
    ) -> Result<()> {
        self.0.remove_topic_member(&topic_id, &user_id).await
    }
}
