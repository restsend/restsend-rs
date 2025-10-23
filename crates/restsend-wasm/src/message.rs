use crate::{
    callback::MessageCallbackWasmWrap,
    js_util::{get_bool, get_string, get_vec_strings},
    Client,
};
use restsend_sdk::models::conversation::Extra;
use wasm_bindgen::prelude::*;

#[allow(non_snake_case)]
#[wasm_bindgen]
impl Client {
    ///
    /// Send message with content
    /// # Arguments
    /// * `topicId` - The topic id
    /// * `content` - The content Object
    ///     * `type` String - The content type, must be [text, image, video, audio, file, YOUR_CUSTOM_TYPE]
    ///     * `text` String - The text message
    ///     * `attachment` Object - The attachment object
    ///     * `duration` String - The duration of the content, only for video and audio, optional, format is hh:mm:ss
    ///     * `thumbnail` Object - The thumbnail object, only for video and image, optional
    ///     * `size` Number - The size of the content, only for file, optional
    ///     * `placeholder` String - The placeholder of the content, optional
    ///     * `width` Number - The width of the content, only for image/video, optional
    ///     * `height` Number - The height of the content, only for image/video, optional
    ///     * `reply` String - The reply message id, optional
    ///     * `mentions` Array - Mention to users, optional
    ///     * `mentionsAll` Boolean - Mention to all users, optional
    /// * `option` - The send option
    /// # Return
    /// The message id
    /// # Example
    /// ```javascript
    /// const client = new Client(info);
    /// await client.connect();
    /// await client.doSend(topicId, {
    ///     type: 'wx.text',
    ///     text: 'hello',
    /// }, {
    ///     mentions: undefined, // The mention user id list, optional
    ///     mentionAll:  false, // Mention all users, optional
    ///     reply:  undefined, // The reply message id, optional
    ///     onsent:  () => {}, // The callback when message sent
    ///     onprogress:  (progress:Number, total:Number)  =>{}, // The callback when message sending progress
    ///     onattachmentupload:  (result:Upload) => { }, // The callback when attachment uploaded, return the Content object to replace the original content
    ///     onack:  (req:ChatRequest)  => {}, // The callback when message acked
    ///     onfail:  (reason:String)  => {} // The callback when message failed
    /// });
    /// ```
    #[cfg(target_family = "wasm")]
    pub async fn doSend(
        &self,
        topicId: String,
        content: JsValue,
        option: JsValue,
    ) -> Result<String, JsValue> {
        let mut content = super::js_util::js_value_to_content(content)?;
        content.mentions = get_vec_strings(&option, "mentions").unwrap_or(content.mentions);
        content.reply = get_string(&option, "reply").unwrap_or(content.reply);
        content.mention_all = get_bool(&option, "mentionAll");

        self.inner
            .do_send(
                topicId,
                content,
                Some(Box::new(MessageCallbackWasmWrap::new(option))),
            )
            .await
            .map_err(|e| e.into())
    }
    /// Send typing status
    /// # Arguments
    /// * `topicId` - The topic id    
    pub async fn doTyping(&self, topicId: String) -> Result<(), JsValue> {
        self.inner.do_typing(topicId).await.map_err(|e| e.into())
    }
    /// Recall message
    /// # Arguments
    /// * `topicId` - The topic id
    /// * `messageId` - The message id
    pub async fn doRecall(
        &self,
        topicId: String,
        messageId: String,
        option: JsValue,
    ) -> Result<String, JsValue> {
        self.inner
            .do_recall(
                topicId,
                messageId,
                Some(Box::new(MessageCallbackWasmWrap::new(option))),
            )
            .await
            .map_err(|e| e.into())
    }
    /// Send voice message
    /// # Arguments
    /// * `topicId` - The topic id
    /// * `attachment` - The attachment object
    /// * `option` - The send option
    ///     * `duration` String - The duration of the content, only for video and audio, optional, format is hh:mm:ss
    ///     * `mentions` Array - The mention user id list, optional
    ///     * `mentionAll` boolean, // Mention all users, optional
    ///     * `reply` String - The reply message id, optional
    /// # Return
    /// The message id
    #[cfg(target_family = "wasm")]
    pub async fn doSendVoice(
        &self,
        topicId: String,
        attachment: JsValue,
        option: JsValue,
    ) -> Result<String, JsValue> {
        self.inner
            .do_send_voice(
                topicId,
                super::js_util::js_value_to_attachment(&attachment)?,
                get_string(&option, "duration").unwrap_or_default(),
                get_vec_strings(&option, "mentions"),
                get_bool(&option, "mentionAll"),
                get_string(&option, "reply"),
                Some(Box::new(MessageCallbackWasmWrap::new(option))),
            )
            .await
            .map_err(|e| e.into())
    }

    /// Send video message
    /// # Arguments
    /// * `topicId` - The topic id
    /// * `attachment` - The attachment object
    /// * `option` - The send option
    ///    * `duration` String - The duration of the content, only for video and audio, optional, format is hh:mm:ss
    ///    * `mentions` Array - The mention user id list, optional
    ///    * `mentionAll` boolean, // Mention all users, optional
    ///    * `reply` String - The reply message id, optional
    /// # Return
    /// The message id
    #[cfg(target_family = "wasm")]
    pub async fn doSendVideo(
        &self,
        topicId: String,
        attachment: JsValue,
        option: JsValue,
    ) -> Result<String, JsValue> {
        self.inner
            .do_send_video(
                topicId,
                super::js_util::js_value_to_attachment(&attachment)?,
                get_string(&option, "duration").unwrap_or_default(),
                get_vec_strings(&option, "mentions"),
                get_bool(&option, "mentionAll"),
                get_string(&option, "reply"),
                Some(Box::new(MessageCallbackWasmWrap::new(option))),
            )
            .await
            .map_err(|e| e.into())
    }

    /// Send file message
    /// # Arguments
    /// * `topicId` - The topic id
    /// * `attachment` - The attachment object
    /// * `option` - The send option
    ///    * `size` Number - The size of the content, only for file, optional
    ///    * `mentions` Array - The mention user id list, optional
    ///    * `mentionAll` boolean, // Mention all users, optional
    ///    * `reply` String - The reply message id, optional
    /// # Return
    /// The message id
    #[cfg(target_family = "wasm")]
    pub async fn doSendFile(
        &self,
        topicId: String,
        attachment: JsValue,
        option: JsValue,
    ) -> Result<String, JsValue> {
        self.inner
            .do_send_file(
                topicId,
                super::js_util::js_value_to_attachment(&attachment)?,
                get_vec_strings(&option, "mentions"),
                get_bool(&option, "mentionAll"),
                get_string(&option, "reply"),
                Some(Box::new(MessageCallbackWasmWrap::new(option))),
            )
            .await
            .map_err(|e| e.into())
    }

    /// Send location message
    /// # Arguments
    /// * `topicId` - The topic id
    /// * `latitude` - The latitude
    /// * `longitude` - The longitude
    /// * `address` - The address
    /// * `option` - The send option
    ///   * `mentions` Array - The mention user id list, optional
    ///   * `mentionAll` boolean, // Mention all users, optional
    ///   * `reply` String - The reply message id, optional
    /// # Return
    /// The message id
    pub async fn doSendLocation(
        &self,
        topicId: String,
        latitude: String,
        longitude: String,
        address: String,
        option: JsValue,
    ) -> Result<String, JsValue> {
        self.inner
            .do_send_location(
                topicId,
                latitude,
                longitude,
                address,
                get_vec_strings(&option, "mentions"),
                get_bool(&option, "mentionAll"),
                get_string(&option, "reply"),
                Some(Box::new(MessageCallbackWasmWrap::new(option))),
            )
            .await
            .map_err(|e| e.into())
    }
    /// Send link message
    /// # Arguments
    /// * `topicId` - The topic id
    /// * `url` - The url
    /// * `option` - The send option
    ///  * `placeholder` String - The placeholder of the content, optional
    ///  * `mentions` Array - The mention user id list, optional
    ///  * `mentionAll` boolean, // Mention all users, optional
    ///  * `reply` String - The reply message id, optional
    /// # Return
    /// The message id
    pub async fn doSendLink(
        &self,
        topicId: String,
        url: String,
        option: JsValue,
    ) -> Result<String, JsValue> {
        self.inner
            .do_send_link(
                topicId,
                url,
                get_string(&option, "placeholder").unwrap_or_default(),
                get_vec_strings(&option, "mentions"),
                get_bool(&option, "mentionAll"),
                get_string(&option, "reply"),
                Some(Box::new(MessageCallbackWasmWrap::new(option))),
            )
            .await
            .map_err(|e| e.into())
    }

    /// Send invite message
    /// # Arguments
    /// * `topicId` - The topic id
    /// * `logIds` Array - The log id list
    /// * `option` - The send option
    /// # Return    
    /// The message id
    pub async fn doSendLogs(
        &self,
        topicId: String,
        sourceTopicId: String,
        logIds: Vec<String>,
        option: JsValue,
    ) -> Result<String, JsValue> {
        self.inner
            .do_send_logs(
                topicId,
                sourceTopicId,
                logIds,
                get_vec_strings(&option, "mentions"),
                get_bool(&option, "mentionAll"),
                Some(Box::new(MessageCallbackWasmWrap::new(option))),
            )
            .await
            .map_err(|e| e.into())
    }

    /// Send text message
    /// # Arguments
    /// * `topicId` - The topic id
    /// * `text` - The text message
    /// * `option` - The send option
    /// # Return
    /// The message id
    /// # Example
    /// ```javascript
    /// const client = new Client(info);
    /// await client.connect();
    /// await client.sendText(topicId, text, {
    ///     mentions: [] || undefined, // The mention user id list, optional
    ///     reply: String || undefined, - The reply message id, optional
    ///     onsent:  () => {},
    ///     onprogress:  (progress:Number, total:Number)  =>{},
    ///     onack:  (req:ChatRequest)  => {},
    ///     onfail:  (reason:String)  => {}
    /// });
    /// ```
    pub async fn doSendText(
        &self,
        topicId: String,
        text: String,
        option: JsValue,
    ) -> Result<String, JsValue> {
        self.inner
            .do_send_text(
                topicId,
                text,
                get_vec_strings(&option, "mentions"),
                get_string(&option, "reply"),
                Some(Box::new(MessageCallbackWasmWrap::new(option))),
            )
            .await
            .map_err(|e| e.into())
    }
    ///
    /// Send image message
    /// # Arguments
    /// * `topicId` - The topic id
    /// * `attachment` - The attachment object
    ///     * `file` File - The file object
    ///     * `url` String  - The file name
    /// * `option` - The send option
    /// # Example
    /// ```javascript
    /// const client = new Client(info);
    /// await client.connect();
    /// await client.sendImage(topicId, {file:new File(['(⌐□_□)'], 'hello_restsend.png', { type: 'image/png' })}, {});
    /// ```
    #[cfg(target_family = "wasm")]
    pub async fn doSendImage(
        &self,
        topicId: String,
        attachment: JsValue,
        option: JsValue,
    ) -> Result<String, JsValue> {
        self.inner
            .do_send_image(
                topicId,
                super::js_util::js_value_to_attachment(&attachment)?,
                get_vec_strings(&option, "mentions"),
                get_string(&option, "reply"),
                Some(Box::new(MessageCallbackWasmWrap::new(option))),
            )
            .await
            .map_err(|e| e.into())
    }
    /// Update sent chat message's extra
    /// # Arguments
    /// * `topicId` - The topic id
    /// * `chatId` - The chat id
    /// * `extra` - The extra, optional
    /// * `option` - The send option
    /// # Return
    /// The message id
    pub async fn doUpdateExtra(
        &self,
        topicId: String,
        chatId: String,
        extra: JsValue,
        option: JsValue,
    ) -> Result<String, JsValue> {
        self.inner
            .do_update_extra(
                topicId,
                chatId,
                serde_wasm_bindgen::from_value::<Extra>(extra).ok(),
                Some(Box::new(MessageCallbackWasmWrap::new(option))),
            )
            .await
            .map_err(|e| e.into())
    }
    /// Send ping message
    /// # Arguments
    /// * `content` - The content string
    /// * `option` - The send option
    /// # Return
    /// The message id
    pub async fn doPing(&self, content: String, option: JsValue) -> Result<String, JsValue> {
        self.inner
            .do_ping(
                content,
                Some(Box::new(MessageCallbackWasmWrap::new(option))),
            )
            .await
            .map_err(|e| e.into())
    }
}
