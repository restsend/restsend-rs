pub(crate) mod test_server;
mod test_client;
mod test_conversation;
mod test_demo_dm;
mod test_local_e2e;
mod test_logs;
mod test_merge_conversation;
mod test_message_conversation;
mod test_reentrancy;
mod test_sync_first_page;
mod test_upload;
mod test_users;

pub(crate) fn unique_test_user(prefix: &str) -> String {
    format!("{}-{}", prefix, crate::utils::random_text(8))
}
