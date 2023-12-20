use crate::{
    callback::MessageCallbackWasmWrap,
    js_util::{get_string, get_vec_strings, js_value_to_attachment, js_value_to_content},
    Client,
};
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
    ///     reply:  undefined, // The reply message id, optional
    ///     onsent:  () => {}, // The callback when message sent
    ///     onprogress:  (progress:Number, total:Number)  =>{}, // The callback when message sending progress
    ///     onack:  (req:ChatRequest)  => {}, // The callback when message acked
    ///     onfail:  (reason:String)  => {} // The callback when message failed
    /// });
    /// ```
    pub async fn doSend(
        &self,
        topicId: String,
        content: JsValue,
        option: JsValue,
    ) -> Result<String, JsValue> {
        let content = js_value_to_content(content)?;
        self.inner
            .do_send(
                topicId,
                content,
                Some(Box::new(MessageCallbackWasmWrap::new(option))),
            )
            .await
            .map_err(|e| e.to_string().into())
    }
    /// Send typing status
    /// # Arguments
    /// * `topicId` - The topic id    
    pub async fn doTyping(&self, topicId: String) -> Result<(), JsValue> {
        self.inner
            .do_typing(topicId)
            .await
            .map_err(|e| e.to_string().into())
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
            .map_err(|e| e.to_string().into())
    }
    /// Send voice message
    /// # Arguments
    /// * `topicId` - The topic id
    /// * `attachment` - The attachment object
    /// * `option` - The send option
    ///     * `duration` String - The duration of the content, only for video and audio, optional, format is hh:mm:ss
    ///     * `mentions` Array - The mention user id list, optional
    ///     * `reply` String - The reply message id, optional
    /// # Return
    /// The message id
    pub async fn doSendVoice(
        &self,
        topicId: String,
        attachment: JsValue,
        option: JsValue,
    ) -> Result<String, JsValue> {
        self.inner
            .do_send_voice(
                topicId,
                js_value_to_attachment(&attachment)?,
                get_string(&option, "duration").unwrap_or_default(),
                get_vec_strings(&option, "mentions"),
                get_string(&option, "reply"),
                Some(Box::new(MessageCallbackWasmWrap::new(option))),
            )
            .await
            .map_err(|e| e.to_string().into())
    }

    /// Send video message
    /// # Arguments
    /// * `topicId` - The topic id
    /// * `attachment` - The attachment object
    /// * `option` - The send option
    ///    * `duration` String - The duration of the content, only for video and audio, optional, format is hh:mm:ss
    ///    * `mentions` Array - The mention user id list, optional
    ///    * `reply` String - The reply message id, optional
    /// # Return
    /// The message id
    pub async fn doSendVideo(
        &self,
        topicId: String,
        attachment: JsValue,
        option: JsValue,
    ) -> Result<String, JsValue> {
        self.inner
            .do_send_video(
                topicId,
                js_value_to_attachment(&attachment)?,
                get_string(&option, "duration").unwrap_or_default(),
                get_vec_strings(&option, "mentions"),
                get_string(&option, "reply"),
                Some(Box::new(MessageCallbackWasmWrap::new(option))),
            )
            .await
            .map_err(|e| e.to_string().into())
    }

    /// Send file message
    /// # Arguments
    /// * `topicId` - The topic id
    /// * `attachment` - The attachment object
    /// * `option` - The send option
    ///    * `size` Number - The size of the content, only for file, optional
    ///    * `mentions` Array - The mention user id list, optional
    ///    * `reply` String - The reply message id, optional
    /// # Return
    /// The message id
    pub async fn doSendFile(
        &self,
        topicId: String,
        attachment: JsValue,
        option: JsValue,
    ) -> Result<String, JsValue> {
        self.inner
            .do_send_file(
                topicId,
                js_value_to_attachment(&attachment)?,
                get_vec_strings(&option, "mentions"),
                get_string(&option, "reply"),
                Some(Box::new(MessageCallbackWasmWrap::new(option))),
            )
            .await
            .map_err(|e| e.to_string().into())
    }

    /// Send location message
    /// # Arguments
    /// * `topicId` - The topic id
    /// * `latitude` - The latitude
    /// * `longitude` - The longitude
    /// * `address` - The address
    /// * `option` - The send option
    ///   * `mentions` Array - The mention user id list, optional
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
                get_string(&option, "reply"),
                Some(Box::new(MessageCallbackWasmWrap::new(option))),
            )
            .await
            .map_err(|e| e.to_string().into())
    }
    /// Send link message
    /// # Arguments
    /// * `topicId` - The topic id
    /// * `url` - The url
    /// * `option` - The send option
    ///  * `placeholder` String - The placeholder of the content, optional
    ///  * `mentions` Array - The mention user id list, optional
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
                get_string(&option, "reply"),
                Some(Box::new(MessageCallbackWasmWrap::new(option))),
            )
            .await
            .map_err(|e| e.to_string().into())
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
        logIds: Vec<String>,
        option: JsValue,
    ) -> Result<String, JsValue> {
        self.inner
            .do_send_logs(
                topicId,
                logIds,
                get_vec_strings(&option, "mentions"),
                Some(Box::new(MessageCallbackWasmWrap::new(option))),
            )
            .await
            .map_err(|e| e.to_string().into())
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
            .map_err(|e| e.to_string().into())
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
    pub async fn doSendImage(
        &self,
        topicId: String,
        attachment: JsValue,
        option: JsValue,
    ) -> Result<String, JsValue> {
        self.inner
            .do_send_image(
                topicId,
                js_value_to_attachment(&attachment)?,
                get_vec_strings(&option, "mentions"),
                get_string(&option, "reply"),
                Some(Box::new(MessageCallbackWasmWrap::new(option))),
            )
            .await
            .map_err(|e| e.to_string().into())
    }
}
