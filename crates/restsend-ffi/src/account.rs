use restsend_sdk::{account, models::AuthInfo, services, utils, Result};

#[uniffi::export]
pub fn init_log(level: String, is_test: bool) {
    utils::init_log(&level, is_test)
}

#[uniffi::export]
pub fn get_current_user(root_path: String) -> Option<AuthInfo> {
    account::get_current_user(&root_path)
}

#[uniffi::export]
pub fn set_current_user(root_path: String, user_id: String) {
    account::set_current_user(&root_path, &user_id).ok();
}

#[uniffi::export]
pub async fn login(endpoint: String, user_id: String, password: String) -> Result<AuthInfo> {
    services::auth::login_with_password(&endpoint, &user_id, &password).await
}
