use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::{
    callback,
    client::{tests::TEST_ENDPOINT, Client},
    models::GetChatLogsResult,
    request::ChatRequest,
    services::auth::login_with_password,
    utils::{check_until, init_log},
};

#[tokio::test]
async fn test_client_fetch_logs() {
    init_log("INFO", true);
    let info = login_with_password(TEST_ENDPOINT, "bob", "bob:demo").await;
    let c = Client::new("", "", &info.unwrap());
    let topic_id = "bob:alice";

    let local_logs = c.store.get_chat_logs("bob:alice", 0, 10).unwrap();
    assert_eq!(local_logs.items.len(), 0);

    let req = ChatRequest::new_text(topic_id, "hello via test_client_fetch_logs");
    let resp = c.send_chat_request(topic_id, req).await.unwrap();

    struct TestSyncLogsCallbackImpl {
        result: Arc<Mutex<Option<GetChatLogsResult>>>,
    }

    impl callback::SyncChatLogsCallback for TestSyncLogsCallbackImpl {
        fn on_success(&self, r: GetChatLogsResult) {
            let mut result = self.result.lock().unwrap();
            result.replace(r);
        }
    }
    let result = Arc::new(Mutex::new(None));

    let cb = TestSyncLogsCallbackImpl {
        result: result.clone(),
    };

    c.sync_chat_logs("bob:alice", 0, 10, Box::new(cb));

    check_until(Duration::from_secs(3), || result.lock().unwrap().is_some())
        .await
        .unwrap();

    let r = result.lock().unwrap().take().unwrap();
    assert!(r.start_seq >= resp.seq);

    let local_logs = c.store.get_chat_logs("bob:alice", 0, 10).unwrap();
    assert_eq!(local_logs.items.len(), 10);

    assert_eq!(r.start_seq, local_logs.start_sort_value);
    assert_eq!(r.end_seq, local_logs.end_sort_value);
}
