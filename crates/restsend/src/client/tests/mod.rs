mod test_client;
mod test_conversation;
mod test_local_e2e;
mod test_logs;
mod test_merge_conversation;
mod test_upload;
mod test_users;

pub(crate) fn test_endpoint() -> String {
    let _ = dotenvy::dotenv();
    std::env::var("RESTSEND_TEST_ENDPOINT").unwrap_or_else(|_| "http://127.0.0.1:8080".to_string())
}

pub(crate) fn unique_test_user(prefix: &str) -> String {
    format!("{}-{}", prefix, crate::utils::random_text(8))
}
