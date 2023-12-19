use super::Client;
use wasm_bindgen::prelude::*;

#[allow(non_snake_case)]
#[wasm_bindgen]
impl Client {
    /// Get user info
    /// #Arguments
    /// * `userId` - user id
    /// * `blocking` - blocking fetch from server
    /// #Return
    /// User info
    pub async fn getUser(&self, userId: String, blocking: Option<bool>) -> JsValue {
        self.inner
            .get_user(userId, blocking.unwrap_or_default())
            .await
            .map(|v| serde_wasm_bindgen::to_value(&v).expect("get_user failed"))
            .unwrap_or(JsValue::UNDEFINED)
    }
    /// Get multiple users info
    /// #Arguments
    /// * `userIds` - Array of user id
    /// #Return
    /// Array of user info
    pub async fn getUsers(&self, userIds: Vec<String>) -> JsValue {
        let users = self.inner.get_users(userIds).await;
        serde_wasm_bindgen::to_value(&users).expect("get_users failed")
    }
    /// Set user remark name
    /// #Arguments
    /// * `userId` - user id
    /// * `remark` - remark name
    pub async fn setUserRemark(&self, userId: String, remark: String) -> Result<(), JsValue> {
        self.inner
            .set_user_remark(userId, remark)
            .await
            .map_err(|e| JsValue::from(e.to_string()))
    }
    /// Set user star
    /// #Arguments
    /// * `userId` - user id
    /// * `star` - star
    pub async fn setUserStar(&self, userId: String, star: bool) -> Result<(), JsValue> {
        self.inner
            .set_user_star(userId, star)
            .await
            .map_err(|e| JsValue::from(e.to_string()))
    }
    /// Set user block
    /// #Arguments
    /// * `userId` - user id
    /// * `block` - block
    pub async fn setUserBlock(&self, userId: String, block: bool) -> Result<(), JsValue> {
        self.inner
            .set_user_block(userId, block)
            .await
            .map_err(|e| JsValue::from(e.to_string()))
    }

    /// Set allow guest chat
    /// #Arguments
    /// * `allow` - allow
    pub async fn setAllowGuestChat(&self, allow: bool) -> Result<(), JsValue> {
        self.inner
            .set_allow_guest_chat(allow)
            .await
            .map_err(|e| JsValue::from(e.to_string()))
    }
}
