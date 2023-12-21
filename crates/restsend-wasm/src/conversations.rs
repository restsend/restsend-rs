use super::Client;
use crate::{
    callback::{SyncChatLogsCallbackWasmWrap, SyncConversationsCallbackWasmWrap},
    js_util::{self, get_string},
};
use restsend_sdk::models::conversation::{Extra, Tags};
use wasm_bindgen::prelude::*;

#[allow(non_snake_case)]
#[wasm_bindgen]
impl Client {
    /// Create a new chat with userId
    /// return: Conversation
    pub async fn createChat(&self, userId: String) -> Option<JsValue> {
        self.inner
            .create_chat(userId)
            .await
            .map(|v| serde_wasm_bindgen::to_value(&v).expect("create_chat failed"))
    }

    /// Clean history of a topic
    pub async fn cleanHistory(&self, topicId: String) -> Result<(), JsValue> {
        self.inner
            .clean_history(topicId)
            .await
            .map_err(|e| JsValue::from(e.to_string()))
    }

    /// Remove messages from a topic
    pub async fn removeMessages(
        &self,
        topicId: String,
        chatIds: Vec<String>,
    ) -> Result<(), JsValue> {
        self.inner
            .remove_messages(topicId, chatIds, true)
            .await
            .map_err(|e| JsValue::from(e.to_string()))
    }

    /// Sync chat logs from server
    /// #Arguments
    /// * `topicId` - topic id
    /// * `lastSeq` - Number, last seq
    /// * `option` - option
    ///     * `limit` - limit
    ///     * `onsuccess` - onsuccess callback -> function (result: GetChatLogsResult)
    ///     * `onerror` - onerror callback -> function (error: String)
    pub async fn syncChatLogs(&self, topicId: String, lastSeq: JsValue, option: JsValue) {
        let limit = js_util::get_f64(&option, "limit") as u32;
        self.inner.sync_chat_logs(
            topicId,
            lastSeq.as_f64().unwrap_or_default() as i64,
            limit,
            Box::new(SyncChatLogsCallbackWasmWrap::new(option)),
        )
    }

    /// Sync conversations from server
    /// #Arguments
    /// * `option` - option
    ///    * `limit` - limit
    ///    * `updatedAt` String - updated_at optional
    ///    * `onsuccess` - onsuccess callback -> function (updated_at:String, count: u32)
    ///         - updated_at: last updated_at
    ///         - count: count of conversations, if count == limit, there may be more conversations, you can call syncConversations again with updated_at, stop when count < limit
    ///    * `onerror` - onerror callback -> function (error: String)
    pub async fn syncConversations(&self, option: JsValue) {
        let limit = js_util::get_f64(&option, "limit") as u32;
        self.inner.sync_conversations(
            get_string(&option, "updatedAt"),
            limit,
            Box::new(SyncConversationsCallbackWasmWrap::new(option)),
        )
    }
    /// Get conversation by topicId
    /// #Arguments
    /// * `topicId` - topic id
    /// return: Conversation or null
    pub fn getConversation(&self, topicId: String) -> JsValue {
        self.inner
            .get_conversation(topicId)
            .map(|v| serde_wasm_bindgen::to_value(&v).expect("get_conversation failed"))
            .unwrap_or(JsValue::UNDEFINED)
    }

    /// Remove conversation by topicId
    /// #Arguments
    /// * `topicId` - topic id
    pub async fn removeConversation(&self, topicId: String) {
        self.inner.remove_conversation(topicId).await
    }

    /// Set conversation sticky by topicId
    /// #Arguments
    /// * `topicId` - topic id
    /// * `sticky` - sticky
    pub async fn setConversationSticky(&self, topicId: String, sticky: bool) {
        self.inner.set_conversation_sticky(topicId, sticky).await
    }

    /// Set conversation mute by topicId
    /// #Arguments
    /// * `topicId` - topic id
    /// * `mute` - mute
    pub async fn setConversationMute(&self, topicId: String, mute: bool) {
        self.inner.set_conversation_mute(topicId, mute).await
    }

    /// Set conversation read by topicId
    /// #Arguments
    /// * `topicId` - topic id
    pub async fn setConversationRead(&self, topicId: String) {
        self.inner.set_conversation_read(topicId).await
    }

    /// Set conversation tags
    /// #Arguments
    /// * `topicId` - topic id
    /// * `tags` - tags is array of Tag:
    ///     - id - string
    ///     - type - string
    ///     - label - string
    pub async fn setConversationTags(&self, topicId: String, tags: JsValue) {
        let tags = serde_wasm_bindgen::from_value::<Tags>(tags).ok();
        self.inner.set_conversation_tags(topicId, tags).await
    }

    /// Set conversation extra
    /// #Arguments
    /// * `topicId` - topic id
    pub async fn setConversationExtra(&self, topicId: String, extra: JsValue) {
        let extra = serde_wasm_bindgen::from_value::<Extra>(extra).ok();
        self.inner.set_conversation_extra(topicId, extra).await
    }
}
