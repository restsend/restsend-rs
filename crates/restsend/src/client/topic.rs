use restsend_macros::export_wasm_or_ffi;

use super::Client;
use crate::models::{Conversation, ListUserResult, Topic, TopicKnock, User};
use crate::services::topic::{create_topic, get_topic, get_topic_members, join_topic};
use crate::services::topic_admin::*;
use crate::Result;

#[export_wasm_or_ffi]
impl Client {
    // Topic API
    pub async fn create_topic(
        &self,
        members: Vec<String>,
        icon: Option<String>,
        name: Option<String>,
    ) -> Result<Conversation> {
        create_topic(&self.endpoint, &self.token, members, icon, name)
            .await
            .map(|t| Conversation::from(&t))
    }

    pub async fn join_topic(
        &self,
        topic_id: String,
        message: Option<String>,
        source: Option<String>,
    ) -> Result<()> {
        join_topic(
            &self.endpoint,
            &self.token,
            &topic_id,
            &message.unwrap_or_default(),
            &source.unwrap_or_default(),
        )
        .await
    }

    pub async fn get_topic(&self, topic_id: String) -> Option<Topic> {
        get_topic(&self.endpoint, &self.token, &topic_id).await.ok()
    }

    pub async fn get_topic_admins(&self, topic_id: String) -> Option<Vec<User>> {
        match self.get_topic(topic_id).await {
            Some(t) => self.store.fetch_users(t.admins).await.ok(),
            None => None,
        }
    }

    pub async fn get_topic_owner(&self, topic_id: String) -> Option<User> {
        match self.get_topic(topic_id).await {
            Some(t) => self.store.fetch_user(&t.owner_id).await.ok(),
            None => None,
        }
    }

    pub async fn get_topic_members(
        &self,
        topic_id: String,
        updated_at: String,
        limit: u32,
    ) -> Option<ListUserResult> {
        get_topic_members(&self.endpoint, &self.token, &topic_id, &updated_at, limit)
            .await
            .ok()
    }

    pub async fn get_topic_knocks(&self, topic_id: String) -> Option<Vec<TopicKnock>> {
        get_topic_knocks(&self.endpoint, &self.token, &topic_id)
            .await
            .ok()
    }

    pub async fn update_topic(
        &self,
        topic_id: String,
        name: Option<String>,
        icon: Option<String>,
    ) -> Result<()> {
        update_topic(&self.endpoint, &self.token, &topic_id, name, icon).await
    }

    pub async fn update_topic_notice(&self, topic_id: String, text: String) -> Result<()> {
        update_topic_notice(&self.endpoint, &self.token, &topic_id, &text).await
    }

    pub async fn silent_topic(&self, topic_id: String, duration: Option<String>) -> Result<()> {
        silent_topic(&self.endpoint, &self.token, &topic_id, duration).await
    }

    pub async fn silent_topic_member(
        &self,
        topic_id: String,
        user_id: String,
        duration: Option<String>,
    ) -> Result<()> {
        silent_topic_member(&self.endpoint, &self.token, &topic_id, &user_id, duration).await
    }

    pub async fn add_topic_admin(&self, topic_id: String, user_id: String) -> Result<()> {
        add_topic_admin(&self.endpoint, &self.token, &topic_id, &user_id).await
    }

    pub async fn remove_topic_admin(&self, topic_id: String, user_id: String) -> Result<()> {
        remove_topic_admin(&self.endpoint, &self.token, &topic_id, &user_id).await
    }

    pub async fn transfer_topic(&self, topic_id: String, user_id: String) -> Result<()> {
        transfer_topic(&self.endpoint, &self.token, &topic_id, &user_id).await
    }

    pub async fn quit_topic(&self, topic_id: String) -> Result<()> {
        quit_topic(&self.endpoint, &self.token, &topic_id).await
    }

    pub async fn dismiss_topic(&self, topic_id: String) -> Result<()> {
        dismiss_topic(&self.endpoint, &self.token, &topic_id).await
    }

    pub async fn accept_topic_join(
        &self,
        topic_id: String,
        user_id: String,
        memo: Option<String>,
    ) -> Result<()> {
        accept_topic_join(&self.endpoint, &self.token, &topic_id, &user_id, memo).await
    }

    pub async fn decline_topic_join(
        &self,
        topic_id: String,
        user_id: String,
        message: Option<String>,
    ) -> Result<()> {
        decline_topic_join(&self.endpoint, &self.token, &topic_id, &user_id, message).await
    }

    pub async fn remove_topic_member(&self, topic_id: String, user_id: String) -> Result<()> {
        remove_topic_member(&self.endpoint, &self.token, &topic_id, &user_id).await
    }
}
