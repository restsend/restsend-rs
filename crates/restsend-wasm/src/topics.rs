use crate::js_util::get_string;

use super::Client;
use wasm_bindgen::prelude::*;

#[allow(non_snake_case)]
#[wasm_bindgen]
impl Client {
    /// Create a new topic
    /// #Arguments
    //    members: Vec<String>,
    ///   name: String,
    ///  icon: String,
    /// #Return
    /// * `Topic` || `undefined`
    pub async fn createTopic(
        &self,
        members: Vec<String>,
        name: Option<String>,
        icon: Option<String>,
    ) -> JsValue {
        self.inner
            .create_topic(members, name, icon)
            .await
            .map(|v| serde_wasm_bindgen::to_value(&v).expect("create_topic failed"))
            .unwrap_or(JsValue::UNDEFINED)
    }

    /// Join a topic
    /// #Arguments
    /// * `topicId` - topic id
    /// * `message` - message
    /// * `source` - source
    pub async fn joinTopic(
        &self,
        topicId: String,
        message: Option<String>,
        source: Option<String>,
    ) -> JsValue {
        self.inner
            .join_topic(topicId, message, source)
            .await
            .map(|_| JsValue::UNDEFINED)
            .unwrap_or(JsValue::UNDEFINED)
    }

    /// Get topic info
    /// #Arguments
    /// * `topicId` - topic id
    /// #Return
    /// * `Topic` || `undefined`
    pub async fn getTopic(&self, topicId: String) -> JsValue {
        self.inner
            .get_topic(topicId)
            .await
            .and_then(|v| Some(serde_wasm_bindgen::to_value(&v).expect("get_topic failed")))
            .unwrap_or(JsValue::UNDEFINED)
    }
    /// Get topic admins
    /// #Arguments
    /// * `topicId` - topic id
    /// #Return
    /// * `Vec<User>` || `undefined`
    pub async fn getTopicAdmins(&self, topicId: String) -> JsValue {
        self.inner
            .get_topic_admins(topicId)
            .await
            .and_then(|v| Some(serde_wasm_bindgen::to_value(&v).expect("get_topic_admins failed")))
            .unwrap_or(JsValue::UNDEFINED)
    }
    /// Get topic owner
    /// #Arguments
    /// * `topicId` - topic id
    /// #Return
    /// * `User` || `undefined`
    pub async fn getTopicOwner(&self, topicId: String) -> JsValue {
        self.inner
            .get_topic_owner(topicId)
            .await
            .and_then(|v| Some(serde_wasm_bindgen::to_value(&v).expect("get_topic_owner failed")))
            .unwrap_or(JsValue::UNDEFINED)
    }
    /// Get topic members
    /// #Arguments
    /// * `topicId` - topic id
    /// * `updatedAt` - updated_at
    /// * `limit` - limit
    /// #Return
    /// * `ListUserResult` || `undefined`
    pub async fn getTopicMembers(&self, topicId: String, updatedAt: String, limit: u32) -> JsValue {
        self.inner
            .get_topic_members(topicId, updatedAt, limit)
            .await
            .and_then(|v| Some(serde_wasm_bindgen::to_value(&v).expect("get_topic_members failed")))
            .unwrap_or(JsValue::UNDEFINED)
    }
    /// Get topic knocks
    /// #Arguments
    /// * `topicId` - topic id
    /// #Return
    /// * `Vec<TopicKnock>`
    pub async fn getTopicKnocks(&self, topicId: String) -> JsValue {
        self.inner
            .get_topic_knocks(topicId)
            .await
            .and_then(|v| Some(serde_wasm_bindgen::to_value(&v).expect("get_topic_knocks failed")))
            .unwrap_or(JsValue::UNDEFINED)
    }
    /// Update topic info
    /// #Arguments
    /// * `topicId` - topic id
    /// * `option` - option
    ///     * `name` - String
    ///     * `icon` - String (url) or base64
    pub async fn updateTopic(&self, topicId: String, option: JsValue) -> Result<(), JsValue> {
        self.inner
            .update_topic(
                topicId,
                get_string(&option, "name"),
                get_string(&option, "icon"),
            )
            .await
            .map_err(|e| JsValue::from(e.to_string()))
    }
    /// Update topic notice
    /// #Arguments
    /// * `topicId` - topic id
    /// * `text` - notice text
    pub async fn updateTopicNotice(&self, topicId: String, text: String) -> Result<(), JsValue> {
        self.inner
            .update_topic_notice(topicId, text)
            .await
            .map_err(|e| JsValue::from(e.to_string()))
    }

    /// Silence topic
    /// #Arguments
    /// * `topicId` - topic id
    /// * `duration` - duration, format: 1d, 1h, 1m, cancel with empty string
    pub async fn silentTopic(
        &self,
        topicId: String,
        duration: Option<String>,
    ) -> Result<(), JsValue> {
        self.inner
            .silent_topic(topicId, duration)
            .await
            .map_err(|e| JsValue::from(e.to_string()))
    }

    /// Silent topic member
    /// #Arguments
    /// * `topicId` - topic id
    /// * `userId` - user id
    /// * `duration` - duration, format: 1d, 1h, 1m, cancel with empty string
    pub async fn silentTopicMember(
        &self,
        topicId: String,
        userId: String,
        duration: Option<String>,
    ) -> Result<(), JsValue> {
        self.inner
            .silent_topic_member(topicId, userId, duration)
            .await
            .map_err(|e| JsValue::from(e.to_string()))
    }

    /// Add topic admin
    /// #Arguments
    /// * `topicId` - topic id
    /// * `userId` - user id
    pub async fn addTopicAdmin(&self, topicId: String, userId: String) -> Result<(), JsValue> {
        self.inner
            .add_topic_admin(topicId, userId)
            .await
            .map_err(|e| JsValue::from(e.to_string()))
    }

    /// Remove topic admin
    /// #Arguments
    /// * `topicId` - topic id
    /// * `userId` - user id
    pub async fn removeTopicAdmin(&self, topicId: String, userId: String) -> Result<(), JsValue> {
        self.inner
            .remove_topic_admin(topicId, userId)
            .await
            .map_err(|e| JsValue::from(e.to_string()))
    }

    /// Transfer topic
    /// #Arguments
    /// * `topicId` - topic id
    /// * `userId` - user id to transfer, the user must be a topic member
    pub async fn transferTopic(&self, topicId: String, userId: String) -> Result<(), JsValue> {
        self.inner
            .transfer_topic(topicId, userId)
            .await
            .map_err(|e| JsValue::from(e.to_string()))
    }

    /// Quit topic
    /// #Arguments
    /// * `topicId` - topic id
    pub async fn quitTopic(&self, topicId: String) -> Result<(), JsValue> {
        self.inner
            .quit_topic(topicId)
            .await
            .map_err(|e| JsValue::from(e.to_string()))
    }

    /// Dismiss topic
    /// #Arguments
    /// * `topicId` - topic id
    pub async fn dismissTopic(&self, topicId: String) -> Result<(), JsValue> {
        self.inner
            .dismiss_topic(topicId)
            .await
            .map_err(|e| JsValue::from(e.to_string()))
    }

    /// Accept topic join
    /// #Arguments
    /// * `topicId` - topic id
    /// * `userId` - user id
    /// * `memo` - accept memo
    pub async fn acceptTopicJoin(
        &self,
        topicId: String,
        userId: String,
        memo: Option<String>,
    ) -> Result<(), JsValue> {
        self.inner
            .accept_topic_join(topicId, userId, memo)
            .await
            .map_err(|e| JsValue::from(e.to_string()))
    }

    /// Decline topic join
    /// #Arguments
    /// * `topicId` - topic id
    /// * `userId` - user id
    /// * `message` - decline message
    pub async fn declineTopicJoin(
        &self,
        topicId: String,
        userId: String,
        message: Option<String>,
    ) -> Result<(), JsValue> {
        self.inner
            .decline_topic_join(topicId, userId, message)
            .await
            .map_err(|e| JsValue::from(e.to_string()))
    }

    /// Remove topic member
    /// #Arguments
    /// * `topicId` - topic id
    /// * `userId` - user id
    pub async fn removeTopicMember(&self, topicId: String, userId: String) -> Result<(), JsValue> {
        self.inner
            .remove_topic_member(topicId, userId)
            .await
            .map_err(|e| JsValue::from(e.to_string()))
    }
}
