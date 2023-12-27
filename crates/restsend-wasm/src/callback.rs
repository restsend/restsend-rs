use crate::{js_util::get_function, CallbackFunction, Client};
use restsend_sdk::{
    models::{Content, Conversation},
    request::ChatRequest,
    services::response::Upload,
};
use serde::Serialize;
use wasm_bindgen::prelude::*;

pub(super) struct MessageCallbackWasmWrap {
    pub(super) cb_on_sent: CallbackFunction,
    pub(super) cb_on_progress: CallbackFunction,
    pub(super) cb_on_attachment_upload: CallbackFunction,
    pub(super) cb_on_ack: CallbackFunction,
    pub(super) cb_on_fail: CallbackFunction,
}

unsafe impl Send for MessageCallbackWasmWrap {}
unsafe impl Sync for MessageCallbackWasmWrap {}

impl MessageCallbackWasmWrap {
    pub fn new(cb: JsValue) -> Self {
        Self {
            cb_on_sent: get_function(&cb, "onsent"),
            cb_on_progress: get_function(&cb, "onprogress"),
            cb_on_attachment_upload: get_function(&cb, "onattachmentupload"),
            cb_on_ack: get_function(&cb, "onack"),
            cb_on_fail: get_function(&cb, "onfail"),
        }
    }
}

impl restsend_sdk::callback::MessageCallback for MessageCallbackWasmWrap {
    fn on_sent(&self) {
        if let Some(cb) = self.cb_on_sent.lock().unwrap().as_ref() {
            cb.call0(&JsValue::NULL)
                .err()
                .map(|e| web_sys::console::error_1(&e));
        }
    }

    fn on_progress(&self, progress: u64, total: u64) {
        if let Some(cb) = self.cb_on_progress.lock().unwrap().as_ref() {
            cb.call2(
                &JsValue::NULL,
                &JsValue::from_f64(progress as f64),
                &JsValue::from_f64(total as f64),
            )
            .err()
            .map(|e| web_sys::console::error_1(&e));
        }
    }

    fn on_attachment_upload(&self, result: Upload) -> Option<Content> {
        if let Some(cb) = self.cb_on_attachment_upload.lock().unwrap().as_ref() {
            let result = serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::UNDEFINED);
            match cb.call1(&JsValue::NULL, &result) {
                Ok(r) => match serde_wasm_bindgen::from_value(r) {
                    Ok(r) => return Some(r),
                    Err(e) => {
                        web_sys::console::error_1(&e.into());
                    }
                },
                Err(e) => {
                    web_sys::console::error_1(&e);
                }
            }
        }
        None
    }

    fn on_ack(&self, req: ChatRequest) {
        if let Some(cb) = self.cb_on_ack.lock().unwrap().as_ref() {
            let serializer = &serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
            let req = req.serialize(serializer).unwrap_or(JsValue::UNDEFINED);
            cb.call1(&JsValue::NULL, &req)
                .err()
                .map(|e| web_sys::console::error_1(&e));
        }
    }

    fn on_fail(&self, reason: String) {
        if let Some(cb) = self.cb_on_fail.lock().unwrap().as_ref() {
            cb.call1(&JsValue::NULL, &JsValue::from_str(&reason))
                .err()
                .map(|e| web_sys::console::error_1(&e));
        }
    }
}

pub(super) struct SyncChatLogsCallbackWasmWrap {
    pub(super) cb_on_success: CallbackFunction,
    pub(super) cb_on_fail: CallbackFunction,
}

unsafe impl Send for SyncChatLogsCallbackWasmWrap {}
unsafe impl Sync for SyncChatLogsCallbackWasmWrap {}

impl SyncChatLogsCallbackWasmWrap {
    pub fn new(cb: JsValue) -> Self {
        Self {
            cb_on_success: get_function(&cb, "onsuccess"),
            cb_on_fail: get_function(&cb, "onfail"),
        }
    }
}

impl restsend_sdk::callback::SyncChatLogsCallback for SyncChatLogsCallbackWasmWrap {
    fn on_success(&self, r: restsend_sdk::models::GetChatLogsResult) {
        if let Some(cb) = self.cb_on_success.lock().unwrap().as_ref() {
            let serializer = &serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
            let r = r.serialize(serializer).unwrap_or(JsValue::UNDEFINED);
            cb.call1(&JsValue::NULL, &r)
                .err()
                .map(|e| web_sys::console::error_1(&e));
        }
    }

    fn on_fail(&self, e: restsend_sdk::Error) {
        if let Some(cb) = self.cb_on_fail.lock().unwrap().as_ref() {
            let e = serde_wasm_bindgen::to_value(&e.to_string()).unwrap_or(JsValue::UNDEFINED);
            cb.call1(&JsValue::NULL, &e)
                .err()
                .map(|e| web_sys::console::error_1(&e));
        }
    }
}

pub(super) struct SyncConversationsCallbackWasmWrap {
    pub(super) cb_on_success: CallbackFunction,
    pub(super) cb_on_fail: CallbackFunction,
}

unsafe impl Send for SyncConversationsCallbackWasmWrap {}
unsafe impl Sync for SyncConversationsCallbackWasmWrap {}

impl SyncConversationsCallbackWasmWrap {
    pub fn new(cb: JsValue) -> Self {
        Self {
            cb_on_success: get_function(&cb, "onsuccess"),
            cb_on_fail: get_function(&cb, "onfail"),
        }
    }
}

impl restsend_sdk::callback::SyncConversationsCallback for SyncConversationsCallbackWasmWrap {
    fn on_success(&self, updated_at: String, count: u32) {
        if let Some(cb) = self.cb_on_success.lock().unwrap().as_ref() {
            let arg1 = JsValue::from_str(&updated_at);
            let arg2 = JsValue::from_f64(count as f64);
            cb.call2(&JsValue::NULL, &arg1, &arg2)
                .err()
                .map(|e| web_sys::console::error_1(&e));
        }
    }

    fn on_fail(&self, e: restsend_sdk::Error) {
        if let Some(cb) = self.cb_on_fail.lock().unwrap().as_ref() {
            let e = serde_wasm_bindgen::to_value(&e.to_string()).unwrap_or(JsValue::UNDEFINED);
            cb.call1(&JsValue::NULL, &e)
                .err()
                .map(|e| web_sys::console::error_1(&e));
        }
    }
}

pub(super) struct CallbackWasmWrap {
    pub(super) cb_on_connected: CallbackFunction,
    pub(super) cb_on_connecting: CallbackFunction,
    pub(super) cb_on_token_expired: CallbackFunction,
    pub(super) cb_on_net_broken: CallbackFunction,
    pub(super) cb_on_kickoff_by_other_client: CallbackFunction,
    pub(super) cb_on_system_request: CallbackFunction,
    pub(super) cb_on_unknown_request: CallbackFunction,
    pub(super) cb_on_topic_typing: CallbackFunction,
    pub(super) cb_on_topic_message: CallbackFunction,
    pub(super) cb_on_topic_read: CallbackFunction,
    pub(super) cb_on_conversations_updated: CallbackFunction,
    pub(super) cb_on_conversation_removed: CallbackFunction,
}
unsafe impl Send for CallbackWasmWrap {}
unsafe impl Sync for CallbackWasmWrap {}

impl restsend_sdk::callback::Callback for CallbackWasmWrap {
    fn on_connected(&self) {
        if let Some(cb) = self.cb_on_connected.lock().unwrap().as_ref() {
            cb.call0(&JsValue::NULL)
                .err()
                .map(|e| web_sys::console::error_1(&e));
        }
    }
    fn on_connecting(&self) {
        if let Some(cb) = self.cb_on_connecting.lock().unwrap().as_ref() {
            cb.call0(&JsValue::NULL)
                .err()
                .map(|e| web_sys::console::error_1(&e));
        }
    }

    fn on_net_broken(&self, reason: String) {
        if let Some(cb) = self.cb_on_net_broken.lock().unwrap().as_ref() {
            cb.call1(&JsValue::NULL, &JsValue::from_str(&reason))
                .err()
                .map(|e| web_sys::console::error_1(&e));
        }
    }

    fn on_kickoff_by_other_client(&self, reason: String) {
        if let Some(cb) = self.cb_on_kickoff_by_other_client.lock().unwrap().as_ref() {
            cb.call1(&JsValue::NULL, &JsValue::from_str(&reason))
                .err()
                .map(|e| web_sys::console::error_1(&e));
        }
    }

    fn on_token_expired(&self, reason: String) {
        if let Some(cb) = self.cb_on_token_expired.lock().unwrap().as_ref() {
            cb.call1(&JsValue::NULL, &JsValue::from_str(&reason))
                .err()
                .map(|e| web_sys::console::error_1(&e));
        }
    }

    fn on_system_request(&self, req: ChatRequest) -> Option<ChatRequest> {
        if let Some(cb) = self.cb_on_system_request.lock().unwrap().as_ref() {
            let serializer = &serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
            let req = req.serialize(serializer).unwrap_or(JsValue::UNDEFINED);
            match cb.call1(&JsValue::NULL, &req) {
                Ok(result) => {
                    if let Ok(result) = serde_wasm_bindgen::from_value(result) {
                        return Some(result);
                    }
                }
                Err(e) => {
                    web_sys::console::error_1(&e);
                }
            }
        }
        None
    }
    fn on_unknown_request(&self, req: ChatRequest) -> Option<ChatRequest> {
        if let Some(cb) = self.cb_on_unknown_request.lock().unwrap().as_ref() {
            let serializer = &serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
            let req = req.serialize(serializer).unwrap_or(JsValue::UNDEFINED);
            match cb.call1(&JsValue::NULL, &req) {
                Ok(result) => {
                    if let Ok(result) = serde_wasm_bindgen::from_value(result) {
                        return Some(result);
                    }
                }
                Err(e) => {
                    web_sys::console::error_1(&e);
                }
            }
        }
        None
    }
    fn on_topic_typing(&self, topic_id: String, message: Option<String>) {
        if let Some(cb) = self.cb_on_topic_typing.lock().unwrap().as_ref() {
            let message = message.unwrap_or_default();
            cb.call2(
                &JsValue::NULL,
                &JsValue::from_str(&topic_id),
                &JsValue::from_str(&message),
            )
            .err()
            .map(|e| web_sys::console::error_1(&e));
        }
    }

    // if return true, will send `has read` to server
    fn on_new_message(&self, topic_id: String, message: ChatRequest) -> bool {
        if let Some(cb) = self.cb_on_topic_message.lock().unwrap().as_ref() {
            let req = serde_wasm_bindgen::to_value(&message).unwrap_or(JsValue::UNDEFINED);
            match cb.call2(&JsValue::NULL, &JsValue::from_str(&topic_id), &req) {
                Ok(result) => {
                    if let Ok(result) = serde_wasm_bindgen::from_value(result) {
                        return result;
                    }
                }
                Err(e) => {
                    web_sys::console::error_1(&e);
                }
            }
        }
        return false;
    }
    fn on_topic_read(&self, topic_id: String, message: ChatRequest) {
        if let Some(cb) = self.cb_on_topic_read.lock().unwrap().as_ref() {
            let req = serde_wasm_bindgen::to_value(&message).unwrap_or(JsValue::UNDEFINED);
            cb.call2(&JsValue::NULL, &JsValue::from_str(&topic_id), &req)
                .err()
                .map(|e| web_sys::console::error_1(&e));
        }
    }
    fn on_conversations_updated(&self, conversations: Vec<Conversation>) {
        if let Some(cb) = self.cb_on_conversations_updated.lock().unwrap().as_ref() {
            let serializer = &serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
            let conversations = conversations
                .serialize(serializer)
                .unwrap_or(JsValue::UNDEFINED);
            cb.call1(&JsValue::NULL, &conversations)
                .err()
                .map(|e| web_sys::console::error_1(&e));
        }
    }
    fn on_conversation_removed(&self, conversatio_id: String) {
        if let Some(cb) = self.cb_on_conversation_removed.lock().unwrap().as_ref() {
            cb.call1(&JsValue::NULL, &JsValue::from_str(&conversatio_id))
                .err()
                .map(|e| web_sys::console::error_1(&e));
        }
    }
}

#[allow(non_snake_case)]
#[wasm_bindgen]
impl Client {
    /// Set the callback when connection connected
    #[wasm_bindgen(setter)]
    pub fn set_onconnected(&self, cb: JsValue) {
        if cb.is_function() {
            self.cb_on_connected
                .lock()
                .unwrap()
                .replace(js_sys::Function::from(cb));
        }
    }
    /// Set the callback when connection connecting
    #[wasm_bindgen(setter)]
    pub fn set_onconnecting(&self, cb: JsValue) {
        if cb.is_function() {
            self.cb_on_connecting
                .lock()
                .unwrap()
                .replace(js_sys::Function::from(cb));
        }
    }
    /// Set the callback when connection token expired
    #[wasm_bindgen(setter)]
    pub fn set_ontokenexpired(&self, cb: JsValue) {
        if cb.is_function() {
            self.cb_on_token_expired
                .lock()
                .unwrap()
                .replace(js_sys::Function::from(cb));
        }
    }
    /// Set the callback when connection broken
    /// # Arguments
    /// * `reason` String - The reason of the connection broken
    /// # Example
    /// ```javascript
    /// const client = new Client(info);
    /// await client.connect();
    /// client.onnetbroken = (reason) => {
    /// console.log(reason);
    /// }
    /// ```
    #[wasm_bindgen(setter)]
    pub fn set_onbroken(&self, cb: JsValue) {
        if cb.is_function() {
            self.cb_on_net_broken
                .lock()
                .unwrap()
                .replace(js_sys::Function::from(cb));
        }
    }
    /// Set the callback when kickoff by other client
    /// # Arguments
    /// * `reason` String - The reason of the kickoff
    /// # Example
    /// ```javascript
    /// const client = new Client(info);
    /// await client.connect();
    /// client.onkickoff = (reason) => {
    /// console.log(reason);
    /// }
    /// ```
    #[wasm_bindgen(setter)]
    pub fn set_onkickoff(&self, cb: JsValue) {
        if cb.is_function() {
            self.cb_on_kickoff_by_other_client
                .lock()
                .unwrap()
                .replace(js_sys::Function::from(cb));
        }
    }
    /// Set the callback when receive system request
    /// # Arguments
    ///  * `req` - The request object, the return value is the response object
    /// # Example
    /// ```javascript
    /// const client = new Client(info);
    /// await client.connect();
    /// client.onsystemrequest = (req) => {
    ///    if (req.type === 'get') {
    ///       return {type:'resp', code: 200}
    ///   }
    /// }
    /// ```
    #[wasm_bindgen(setter)]
    pub fn set_onsystemrequest(&self, cb: JsValue) {
        if cb.is_function() {
            self.cb_on_system_request
                .lock()
                .unwrap()
                .replace(js_sys::Function::from(cb));
        }
    }

    /// Set the callback when receive unknown request
    /// # Arguments
    ///  * `req` - The request object, the return value is the response object
    /// # Example
    /// ```javascript
    /// const client = new Client(info);
    /// await client.connect();
    /// client.onunknownrequest = (req) => {
    ///   if (req.type === 'get') {
    ///      return {type:'resp', code: 200}
    ///  }
    /// }
    #[wasm_bindgen(setter)]
    pub fn set_onunknownrequest(&self, cb: JsValue) {
        if cb.is_function() {
            self.cb_on_unknown_request
                .lock()
                .unwrap()
                .replace(js_sys::Function::from(cb));
        }
    }

    /// Set the callback when receive typing event
    /// # Arguments
    /// * `topicId` String - The topic id
    /// * `message` ChatRequest - The message
    /// # Example
    /// ```javascript
    /// const client = new Client(info);
    /// await client.connect();
    /// client.ontyping = (topicId, message) => {
    ///  console.log(topicId, message);
    /// }
    /// ```
    #[wasm_bindgen(setter)]
    pub fn set_ontopictyping(&self, cb: JsValue) {
        if cb.is_function() {
            self.cb_on_topic_typing
                .lock()
                .unwrap()
                .replace(js_sys::Function::from(cb));
        }
    }
    /// Set the callback when receive new message
    /// # Arguments
    /// * `topicId` String - The topic id
    /// * `message` ChatRequest - The message
    /// # Return
    /// * `true` - If return true, will send `has read` to server
    /// # Example
    /// ```javascript
    /// const client = new Client(info);
    /// await client.connect();
    /// client.ontopicmessage = (topicId, message) => {
    /// console.log(topicId, message);
    /// return true;
    /// }
    /// ```
    #[wasm_bindgen(setter)]
    pub fn set_ontopicmessage(&self, cb: JsValue) {
        if cb.is_function() {
            self.cb_on_topic_message
                .lock()
                .unwrap()
                .replace(js_sys::Function::from(cb));
        }
    }
    /// Set the callback when receive read event
    /// # Arguments
    /// * `topicId` String - The topic id
    /// * `message` ChatRequest - The message
    /// # Example
    /// ```javascript
    /// const client = new Client(info);
    /// await client.connect();
    /// client.ontopicread = (topicId, message) => {
    /// console.log(topicId, message);
    /// }
    /// ```
    #[wasm_bindgen(setter)]
    pub fn set_ontopicread(&self, cb: JsValue) {
        if cb.is_function() {
            self.cb_on_topic_read
                .lock()
                .unwrap()
                .replace(js_sys::Function::from(cb));
        }
    }
    /// Set the callback when conversations updated
    /// # Arguments
    /// * `conversations` - The conversation list
    /// # Example
    /// ```javascript
    /// const client = new Client(info);
    /// await client.connect();
    /// client.onconversationsupdated = (conversations) => {
    /// console.log(conversations);
    /// }
    /// ```
    #[wasm_bindgen(setter)]
    pub fn set_onconversationsupdated(&self, cb: JsValue) {
        if cb.is_function() {
            self.cb_on_conversations_updated
                .lock()
                .unwrap()
                .replace(js_sys::Function::from(cb));
        }
    }
    /// Set the callback when conversations removed
    /// # Arguments
    /// * `conversationId` - The conversation id
    /// # Example
    /// ```javascript
    /// const client = new Client(info);
    /// await client.connect();
    /// client.onconversationsremoved = (conversationId) => {
    /// console.log(conversationId);
    /// }
    /// ```
    #[wasm_bindgen(setter)]
    pub fn set_onconversationsremoved(&self, cb: JsValue) {
        if cb.is_function() {
            self.cb_on_conversation_removed
                .lock()
                .unwrap()
                .replace(js_sys::Function::from(cb));
        }
    }
}
