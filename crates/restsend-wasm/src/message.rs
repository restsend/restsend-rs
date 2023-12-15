use crate::{
    callback::MessageCallbackWasmWrap,
    js_util::{get_string, get_vec_strings, js_value_to_attachment, js_value_to_content},
    Client,
};
use restsend_sdk::{models::Content, request::ChatRequest};
use wasm_bindgen::prelude::*;

#[allow(non_snake_case)]
#[wasm_bindgen]
impl Client {
    ///
    /// Send image message
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
    /// # Example
    /// ```javascript
    /// const client = new Client(endpoint, userId, token);
    /// await client.connect();
    /// await client.sendImage(topicId, {file:new File(['(⌐□_□)'], 'hello_restsend.png', { type: 'image/png' })}, mentions, replyTo, {});
    /// ```
    pub async fn doSend(
        &self,
        topicId: String,
        content: JsValue,
        option: JsValue,
    ) -> Result<(), JsValue> {
        let content = match js_value_to_content(content) {
            Some(v) => v,
            None => return Err(JsValue::from_str("invalid content format")),
        };

        self.inner
            .do_send(
                topicId,
                content,
                Some(Box::new(MessageCallbackWasmWrap::new(option))),
            )
            .await
            .ok();
        Ok(())
    }

    /// Send text message
    /// # Arguments
    /// * `topicId` - The topic id
    /// * `text` - The text message
    /// * `option` - The send option
    /// # Example
    /// ```javascript
    /// const client = new Client(endpoint, userId, token);
    /// await client.connect();
    /// await client.sendText(topicId, text, mentions, replyTo, {
    ///     mentions: [] || undefined, // The mention user id list, optional
    ///     reply: String || undefined, - The reply message id, optional
    ///     onsent:  () => {},
    ///     onprogress:  (progress:Number, total:Number)  =>{},
    ///     onack:  (req:ChatRequest)  => {},
    ///     onfail:  (reason:String)  => {}
    /// });
    /// ```
    ///
    pub async fn doSendText(
        &self,
        topicId: String,
        text: String,
        option: JsValue,
    ) -> Result<(), JsValue> {
        self.inner
            .do_send_text(
                topicId,
                text,
                get_vec_strings(&option, "mentions"),
                get_string(&option, "reply"),
                Some(Box::new(MessageCallbackWasmWrap::new(option))),
            )
            .await
            .ok();
        Ok(())
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
    /// const client = new Client(endpoint, userId, token);
    /// await client.connect();
    /// await client.sendImage(topicId, {file:new File(['(⌐□_□)'], 'hello_restsend.png', { type: 'image/png' })}, mentions, replyTo, {});
    /// ```
    pub async fn doSendImage(
        &self,
        topicId: String,
        attachment: JsValue,
        option: JsValue,
    ) -> Result<(), JsValue> {
        let attachment = match js_value_to_attachment(&attachment) {
            Some(v) => v,
            None => {
                return Err(JsValue::from_str(
                    "invalid format, must has any of {file:File, url:String}",
                ))
            }
        };

        self.inner
            .do_send_image(
                topicId,
                attachment,
                get_vec_strings(&option, "mentions"),
                get_string(&option, "reply"),
                Some(Box::new(MessageCallbackWasmWrap::new(option))),
            )
            .await
            .ok();
        Ok(())
    }
}
