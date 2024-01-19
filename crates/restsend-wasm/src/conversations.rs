use super::Client;
use crate::{
    callback::{SyncChatLogsCallbackWasmWrap, SyncConversationsCallbackWasmWrap},
    js_util::{self, get_string},
};
use restsend_sdk::models::conversation::{Extra, Tags};
use serde::Serialize;
use wasm_bindgen::prelude::*;
#[allow(non_snake_case)]
#[wasm_bindgen]
impl Client {
    /// Create a new chat with userId
    /// return: Conversation    
    pub async fn createChat(&self, userId: String) -> Result<JsValue, JsValue> {
        let serializer = &serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
        self.inner
            .create_chat(userId)
            .await
            .map(|v| v.serialize(serializer).unwrap_or(JsValue::UNDEFINED))
            .map_err(|e| e.into())
    }

    /// Clean history of a conversation
    pub async fn cleanMessages(&self, topicId: String) -> Result<(), JsValue> {
        self.inner
            .clean_messages(topicId)
            .await
            .map_err(|e| e.into())
    }

    /// Remove messages from a conversation
    pub async fn removeMessages(
        &self,
        topicId: String,
        chatIds: Vec<String>,
    ) -> Result<(), JsValue> {
        self.inner
            .remove_messages(topicId, chatIds, true)
            .await
            .map_err(|e| e.into())
    }

    /// Sync chat logs from server
    /// #Arguments
    /// * `topicId` - topic id
    /// * `lastSeq` - Number, last seq
    /// * `option` - option
    ///     * `limit` - limit
    ///     * `onsuccess` - onsuccess callback -> function (result: GetChatLogsResult)
    ///     * `onerror` - onerror callback -> function (error: String)
    pub async fn syncChatLogs(&self, topicId: String, lastSeq: Option<f64>, option: JsValue) {
        let limit = js_util::get_f64(&option, "limit") as u32;
        self.inner
            .sync_chat_logs(
                topicId,
                lastSeq.map(|v| v as i64),
                limit,
                Box::new(SyncChatLogsCallbackWasmWrap::new(option)),
            )
            .await
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
        self.inner
            .sync_conversations(
                get_string(&option, "updatedAt"),
                limit,
                Box::new(SyncConversationsCallbackWasmWrap::new(option)),
            )
            .await
    }
    /// Get conversation by topicId
    /// #Arguments
    /// * `topicId` - topic id
    /// * `blocking` - blocking optional
    /// return: Conversation or null
    pub async fn getConversation(&self, topicId: String, blocking: Option<bool>) -> JsValue {
        let serializer = &serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
        self.inner
            .get_conversation(topicId, blocking.unwrap_or(false))
            .await
            .and_then(|v| v.serialize(serializer).ok())
            .unwrap_or(JsValue::UNDEFINED)
    }

    /// Remove conversation by topicId
    /// #Arguments
    /// * `topicId` - topic id
    pub async fn removeConversation(&self, topicId: String) {
        self.inner.remove_conversation(topicId).await
    }

    /// Set conversation remark
    /// #Arguments
    /// * `topicId` - topic id
    /// * `remark` - remark
    pub async fn setConversationRemark(
        &self,
        topicId: String,
        remark: Option<String>,
    ) -> Result<JsValue, JsValue> {
        let r = self.inner.set_conversation_remark(topicId, remark).await?;
        let serializer = &serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
        r.serialize(serializer).map_err(|e| e.into())
    }

    /// Set conversation sticky by topicId
    /// #Arguments
    /// * `topicId` - topic id
    /// * `sticky` - sticky
    pub async fn setConversationSticky(
        &self,
        topicId: String,
        sticky: bool,
    ) -> Result<JsValue, JsValue> {
        let r = self.inner.set_conversation_sticky(topicId, sticky).await?;
        let serializer = &serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
        r.serialize(serializer).map_err(|e| e.into())
    }

    /// Set conversation mute by topicId
    /// #Arguments
    /// * `topicId` - topic id
    /// * `mute` - mute
    pub async fn setConversationMute(
        &self,
        topicId: String,
        mute: bool,
    ) -> Result<JsValue, JsValue> {
        let r = self.inner.set_conversation_mute(topicId, mute).await?;
        let serializer = &serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
        r.serialize(serializer).map_err(|e| e.into())
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
    pub async fn setConversationTags(
        &self,
        topicId: String,
        tags: JsValue,
    ) -> Result<JsValue, JsValue> {
        let tags = serde_wasm_bindgen::from_value::<Tags>(tags).ok();
        let r = self.inner.set_conversation_tags(topicId, tags).await?;
        let serializer = &serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
        r.serialize(serializer).map_err(|e| e.into())
    }

    /// Set conversation extra
    /// #Arguments
    /// * `topicId` - topic id
    /// # `extra` - extra
    /// # Return: Conversation
    pub async fn setConversationExtra(
        &self,
        topicId: String,
        extra: JsValue,
    ) -> Result<JsValue, JsValue> {
        let extra = serde_wasm_bindgen::from_value::<Extra>(extra).ok();
        let r = self.inner.set_conversation_extra(topicId, extra).await?;
        let serializer = &serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
        r.serialize(serializer).map_err(|e| e.into())
    }

    /// Filter conversation with options
    /// #Arguments
    /// * `predicate` - filter predicate
    ///     -> return true to keep the conversation
    /// #Return Array of Conversation
    /// #Example
    /// ```js
    /// const conversations = client.filterConversation((c) => {
    ///    return c.remark === 'hello'
    /// })
    /// ```
    /// #Example
    /// ```js
    /// const conversations = await client.filterConversation((c) => {
    ///   return c.remark === 'hello' && c.tags && c.tags.some(t => t.label === 'hello')
    /// })
    ///
    pub async fn filterConversation(&self, predicate: JsValue) -> JsValue {
        let predicate = predicate.dyn_into::<js_sys::Function>().ok();

        let items = match self
            .inner
            .filter_conversation(Box::new(move |c| Some(c)))
            .await
        {
            Some(v) => v,
            None => {
                vec![]
            }
        };

        let vals = js_sys::Array::new();
        for item in &items {
            let serializer = &serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
            match item.serialize(serializer) {
                Ok(v) => {
                    predicate
                        .as_ref()
                        .and_then(|f| match f.call1(&JsValue::NULL, &v) {
                            Ok(r) => Some(r),
                            Err(e) => {
                                web_sys::console::error_1(&e);
                                None
                            }
                        })
                        .and_then(|r| {
                            if r.as_bool().unwrap_or(false) {
                                vals.push(&v);
                            }
                            Some(())
                        });
                }
                Err(e) => {
                    log::warn!("serialize conversation error: {:?}", e);
                }
            }
        }
        vals.into()
    }
}
