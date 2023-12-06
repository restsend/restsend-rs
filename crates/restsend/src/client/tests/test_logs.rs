use crate::{
    client::{tests::TEST_ENDPOINT, Client},
    request::ChatRequest,
    services::auth::login_with_password,
    utils::init_log,
};

#[tokio::test]
async fn test_client_fetch_logs() {
    init_log("INFO", true);
    let info = login_with_password(TEST_ENDPOINT, "bob", "bob:demo").await;
    let c = Client::new("", "", &info.unwrap());
    let topic_id = "bob:alice";

    let local_logs = c.store.get_chat_logs("bob:alice", 0, 10).await.unwrap();
    assert_eq!(local_logs.items.len(), 0);

    let req = ChatRequest::new_text(topic_id, "hello via test_client_fetch_logs");
    let resp = c.send_chat_request(topic_id, req).await.unwrap();

    let r = c.get_chat_logs("bob:alice", 0, 10).await.unwrap();
    assert!(r.start_seq >= resp.seq);

    let local_logs = c.store.get_chat_logs("bob:alice", 0, 10).await.unwrap();
    assert_eq!(local_logs.items.len(), 10);

    assert_eq!(r.start_seq, local_logs.start_sort_value);
    assert_eq!(r.end_seq, local_logs.end_sort_value);
}
