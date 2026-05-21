use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, RwLock,
};

use crate::{
    callback,
    client::store::ClientStore,
    models::{Content, Conversation},
    request::ChatRequest,
};

struct TestCallback {
    conv_updated: Arc<AtomicU32>,
}

impl callback::RsCallback for TestCallback {
    fn on_conversations_updated(&self, _conversations: Vec<Conversation>, _total: Option<i64>) {
        self.conv_updated.fetch_add(1, Ordering::Relaxed);
    }
}

/// Helper: create a ChatRequest that simulates an incoming chat message.
fn make_incoming_chat(
    topic_id: &str,
    chat_id: &str,
    seq: i64,
    sender_id: &str,
    text: &str,
) -> ChatRequest {
    ChatRequest {
        req_type: "chat".to_string(),
        chat_id: chat_id.to_string(),
        topic_id: topic_id.to_string(),
        seq,
        attendee: sender_id.to_string(),
        created_at: format!("2026-05-11T{:02}:00:00Z", seq),
        content: Some(Content {
            content_type: "text".to_string(),
            text: text.to_string(),
            unreadable: false,
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Helper: create a ChatRequest that simulates a server response (ack).
fn make_response(
    topic_id: &str,
    chat_id: &str,
    ack_seq: i64,
    sender_id: &str,
    text: &str,
) -> ChatRequest {
    ChatRequest {
        req_type: "resp".to_string(),
        chat_id: chat_id.to_string(),
        topic_id: topic_id.to_string(),
        seq: ack_seq,
        attendee: sender_id.to_string(),
        created_at: format!("2026-05-11T{:02}:00:00Z", ack_seq),
        content: Some(Content {
            content_type: "text".to_string(),
            text: text.to_string(),
            unreadable: false,
            ..Default::default()
        }),
        code: 200,
        ..Default::default()
    }
}

/// Test that receiving a Chat message updates the conversation's
/// lastMessage, lastSeq, and unread count.
#[tokio::test]
async fn test_incoming_chat_updates_conversation() {
    let store = ClientStore::new("", ":memory:", "http://test", "token", "receiver-user");
    let callback: Arc<RwLock<Option<Box<dyn callback::RsCallback>>>> =
        Arc::new(RwLock::new(Some(Box::new(TestCallback {
            conv_updated: Arc::new(AtomicU32::new(0)),
        }))));

    // Insert an initial conversation with last_seq=0, last_read_seq=0, unread=0
    let mut conv = Conversation::new("topic_1");
    conv.owner_id = "receiver-user".to_string();
    let t = store.message_storage.table::<Conversation>().await.unwrap();
    t.set("", "topic_1", Some(&conv)).await.unwrap();
    drop(t);

    // Simulate receiving a chat message from "sender-user"
    let req = make_incoming_chat("topic_1", "chat_1", 1, "sender-user", "Hello");
    let resps = store.process_incoming(req, callback.clone()).await;

    // Verify response sent back
    assert_eq!(resps.len(), 1, "expected 1 response");
    assert!(resps[0].is_some(), "response should be Some");

    // Read conversation and verify updates
    let t = store.message_storage.table::<Conversation>().await.unwrap();
    let updated = t.get("", "topic_1").await.expect("conversation should exist");
    assert_eq!(updated.last_seq, 1, "last_seq should be 1");
    assert_eq!(updated.unread, 1, "unread should be 1");
    assert!(
        updated.last_message.is_some(),
        "last_message should be set"
    );
    assert_eq!(
        updated.last_message.as_ref().unwrap().text, "Hello",
        "last_message text should match"
    );
    assert_eq!(
        updated.last_sender_id, "sender-user",
        "last_sender_id should be sender-user"
    );
    assert_eq!(
        updated.last_message_seq,
        Some(1),
        "last_message_seq should be 1"
    );
}

/// Test that receiving multiple sequential Chat messages
/// increments unread count beyond 1.
#[tokio::test]
async fn test_multiple_chats_increment_unread() {
    let store = ClientStore::new("", ":memory:", "http://test", "token", "receiver-user");
    let callback: Arc<RwLock<Option<Box<dyn callback::RsCallback>>>> =
        Arc::new(RwLock::new(Some(Box::new(TestCallback {
            conv_updated: Arc::new(AtomicU32::new(0)),
        }))));

    let mut conv = Conversation::new("topic_unread");
    conv.owner_id = "receiver-user".to_string();
    let t = store.message_storage.table::<Conversation>().await.unwrap();
    t.set("", "topic_unread", Some(&conv)).await.unwrap();
    drop(t);

    // First message
    let req1 = make_incoming_chat("topic_unread", "chat_1", 1, "sender-user", "First");
    store.process_incoming(req1, callback.clone()).await;

    let t = store.message_storage.table::<Conversation>().await.unwrap();
    let updated = t.get("", "topic_unread").await.unwrap();
    assert_eq!(updated.unread, 1, "unread should be 1 after first message");
    assert_eq!(updated.last_seq, 1);
    drop(t);

    // Second message
    let req2 = make_incoming_chat("topic_unread", "chat_2", 2, "sender-user", "Second");
    store.process_incoming(req2, callback.clone()).await;

    let t = store.message_storage.table::<Conversation>().await.unwrap();
    let updated = t.get("", "topic_unread").await.unwrap();
    assert_eq!(
        updated.unread, 2,
        "unread should be 2 after second message"
    );
    assert_eq!(updated.last_seq, 2);
    assert_eq!(
        updated.last_message.as_ref().unwrap().text, "Second",
        "last_message should be the latest message"
    );
    assert_eq!(updated.last_sender_id, "sender-user");
}

/// Test that receiving a Response (server ack) after sending a message
/// does NOT update the sender's conversation — that's handled by the
/// Chat message echo path (merge_conversation_from_chat).
#[tokio::test]
async fn test_response_does_not_touch_conversation() {
    let store = ClientStore::new("", ":memory:", "http://test", "token", "sender-user");
    let callback: Arc<RwLock<Option<Box<dyn callback::RsCallback>>>> =
        Arc::new(RwLock::new(Some(Box::new(TestCallback {
            conv_updated: Arc::new(AtomicU32::new(0)),
        }))));

    let mut conv = Conversation::new("topic_send");
    conv.owner_id = "sender-user".to_string();
    conv.last_seq = 0;
    let t = store.message_storage.table::<Conversation>().await.unwrap();
    t.set("", "topic_send", Some(&conv)).await.unwrap();
    drop(t);

    // Save as outgoing (local) chat log
    let log_t = store.message_storage.table::<crate::models::ChatLog>().await.unwrap();
    let log = crate::models::ChatLog {
        id: "send_chat_1".to_string(),
        topic_id: "topic_send".to_string(),
        seq: 0,
        sender_id: "sender-user".to_string(),
        created_at: "2026-05-11T01:00:00Z".to_string(),
        content: Content {
            content_type: "text".to_string(),
            text: "Sent msg".to_string(),
            unreadable: false,
            ..Default::default()
        },
        status: crate::models::ChatLogStatus::Sending,
        ..Default::default()
    };
    log_t.set("topic_send", "send_chat_1", Some(&log)).await.unwrap();
    drop(log_t);

    // Simulate server response with ack_seq=5
    let resp = make_response("topic_send", "send_chat_1", 5, "sender-user", "Sent msg");
    let resps = store.process_incoming(resp, callback.clone()).await;

    assert!(resps.is_empty(), "Response should not generate responses");

    // Conversation should NOT have been updated by the Response handler
    let t = store.message_storage.table::<Conversation>().await.unwrap();
    let updated = t.get("", "topic_send").await.expect("conversation should exist");
    assert_eq!(
        updated.last_seq, 0,
        "sender's last_seq should NOT be updated by ACK"
    );
    assert!(
        updated.last_message.is_none(),
        "sender's last_message should NOT be set by ACK"
    );
}

/// Test that unread count works correctly when conversation
/// already has unread>0 and a new message arrives.
#[tokio::test]
async fn test_unread_increments_from_existing_unread() {
    let store = ClientStore::new("", ":memory:", "http://test", "token", "receiver-user");
    let callback: Arc<RwLock<Option<Box<dyn callback::RsCallback>>>> =
        Arc::new(RwLock::new(Some(Box::new(TestCallback {
            conv_updated: Arc::new(AtomicU32::new(0)),
        }))));

    let mut conv = Conversation::new("topic_existing");
    conv.owner_id = "receiver-user".to_string();
    conv.last_seq = 10;
    conv.last_read_seq = 5; // unread messages from seq 6 to 10
    conv.unread = 5;
    let t = store.message_storage.table::<Conversation>().await.unwrap();
    t.set("", "topic_existing", Some(&conv)).await.unwrap();
    drop(t);

    // New message seq=11
    let req = make_incoming_chat("topic_existing", "chat_new", 11, "sender-user", "New msg");
    store.process_incoming(req, callback.clone()).await;

    let t = store.message_storage.table::<Conversation>().await.unwrap();
    let updated = t.get("", "topic_existing").await.unwrap();
    assert_eq!(
        updated.unread, 6,
        "unread should increase from 5 to 6"
    );
    assert_eq!(updated.last_seq, 11);
}

/// Test that a read event from the same user (echo of own set_conversation_read)
/// does NOT advance last_read_seq past last_seq, so subsequent messages
/// still correctly increment unread.
#[tokio::test]
async fn test_own_read_echo_does_not_advance_last_read_seq() {
    let store = ClientStore::new("", ":memory:", "http://test", "token", "bob");
    let callback: Arc<RwLock<Option<Box<dyn callback::RsCallback>>>> =
        Arc::new(RwLock::new(Some(Box::new(TestCallback {
            conv_updated: Arc::new(AtomicU32::new(0)),
        }))));

    // Start with conversation: last_seq=10, last_read_seq=10, unread=0
    let mut conv = Conversation::new("topic_read_echo");
    conv.owner_id = "bob".to_string();
    conv.last_seq = 10;
    conv.last_read_seq = 10;
    conv.unread = 0;
    let t = store.message_storage.table::<Conversation>().await.unwrap();
    t.set("", "topic_read_echo", Some(&conv)).await.unwrap();
    drop(t);

    // Step 1: Simulate a Read echo from bob himself (like server echoing back
    // the read event after set_conversation_read). attendee = "bob" = self.user_id.
    let read_req = ChatRequest {
        req_type: String::from(crate::request::ChatRequestType::Read),
        topic_id: "topic_read_echo".to_string(),
        seq: 10,
        attendee: "bob".to_string(),
        created_at: "2026-05-12T01:00:00Z".to_string(),
        ..Default::default()
    };
    store.process_incoming(read_req, callback.clone()).await;

    // Verify last_read_seq is still 10 (the read echo did NOT advance it)
    let t = store.message_storage.table::<Conversation>().await.unwrap();
    let updated = t.get("", "topic_read_echo").await.unwrap();
    assert_eq!(
        updated.last_read_seq, 10,
        "read echo from self should NOT advance last_read_seq"
    );
    assert_eq!(updated.unread, 0, "unread should stay 0");
    drop(t);

    // Step 2: Now a real new message arrives (from alice, not bob)
    let msg_req = make_incoming_chat("topic_read_echo", "chat_new_1", 11, "alice", "New msg");
    store.process_incoming(msg_req, callback.clone()).await;

    // Verify unread incremented correctly
    let t = store.message_storage.table::<Conversation>().await.unwrap();
    let updated = t.get("", "topic_read_echo").await.unwrap();
    assert_eq!(
        updated.unread, 1,
        "unread should be 1 after new message from alice"
    );
    assert_eq!(updated.last_seq, 11);
    assert_eq!(updated.last_read_seq, 10, "last_read_seq should stay 10");
    drop(t);

    // Step 3: Second message arrives (seq=12)
    let msg_req2 = make_incoming_chat("topic_read_echo", "chat_new_2", 12, "alice", "Second msg");
    store.process_incoming(msg_req2, callback.clone()).await;

    let t = store.message_storage.table::<Conversation>().await.unwrap();
    let updated = t.get("", "topic_read_echo").await.unwrap();
    assert_eq!(
        updated.unread, 2,
        "unread should be 2 after second new message"
    );
    assert_eq!(updated.last_seq, 12);
    assert_eq!(updated.last_read_seq, 10, "last_read_seq should still be 10");
}

/// Test that a read event from ANOTHER user does NOT advance our last_read_seq.
/// Specifically: when Alice reads Bob's message, Bob's client receives a "read"
/// broadcast. Bob must NOT update his last_read_seq with Alice's seq.
#[tokio::test]
async fn test_other_user_read_does_not_advance_our_last_read_seq() {
    let store = ClientStore::new("", ":memory:", "http://test", "token", "bob");
    let callback: Arc<RwLock<Option<Box<dyn callback::RsCallback>>>> =
        Arc::new(RwLock::new(Some(Box::new(TestCallback {
            conv_updated: Arc::new(AtomicU32::new(0)),
        }))));

    // Bob's conversation: last_seq=20, last_read_seq=15, unread=5
    let mut conv = Conversation::new("topic_other_read");
    conv.owner_id = "bob".to_string();
    conv.last_seq = 20;
    conv.last_read_seq = 15;
    conv.unread = 5;
    let t = store.message_storage.table::<Conversation>().await.unwrap();
    t.set("", "topic_other_read", Some(&conv)).await.unwrap();
    drop(t);

    // Simulate a Read event from Alice (attendee='alice'), like the server
    // broadcasting Alice's read to Bob. attendee != self.user_id ('bob'),
    // so set_conversation_read_local must NOT be called.
    let alice_read = ChatRequest {
        req_type: String::from(crate::request::ChatRequestType::Read),
        topic_id: "topic_other_read".to_string(),
        seq: 20,  // Alice read up to seq 20
        attendee: "alice".to_string(),
        created_at: "2026-05-12T02:00:00Z".to_string(),
        ..Default::default()
    };
    store.process_incoming(alice_read, callback.clone()).await;

    // Bob's last_read_seq and unread must NOT change
    let t = store.message_storage.table::<Conversation>().await.unwrap();
    let updated = t.get("", "topic_other_read").await.unwrap();
    assert_eq!(
        updated.last_read_seq, 15,
        "Bob's last_read_seq must NOT be changed by Alice's read event"
    );
    assert_eq!(
        updated.unread, 5,
        "Bob's unread must NOT be changed by Alice's read event"
    );

    // A new message from Alice should still correctly increment unread
    drop(t);
    let msg_req = make_incoming_chat("topic_other_read", "chat_new", 21, "alice", "New msg");
    store.process_incoming(msg_req, callback.clone()).await;

    let t = store.message_storage.table::<Conversation>().await.unwrap();
    let updated = t.get("", "topic_other_read").await.unwrap();
    assert_eq!(updated.unread, 6, "unread should increase from 5 to 6");
    assert_eq!(updated.last_seq, 21);
    assert_eq!(updated.last_read_seq, 15, "last_read_seq must still be 15");
}

/// Test that merge_conversation_from_chat sets is_partial = false on a
/// newly created conversation (not previously in DB).
#[tokio::test]
async fn test_incoming_chat_sets_is_partial_false() {
    let store = ClientStore::new("", ":memory:", "http://test", "token", "receiver-user");
    let callback: Arc<RwLock<Option<Box<dyn callback::RsCallback>>>> =
        Arc::new(RwLock::new(Some(Box::new(TestCallback {
            conv_updated: Arc::new(AtomicU32::new(0)),
        }))));

    // Conversation does NOT exist in DB. The incoming chat will create it.
    let req = make_incoming_chat("topic_partial", "chat_1", 1, "sender-user", "Hello");
    store.process_incoming(req, callback.clone()).await;

    let t = store.message_storage.table::<Conversation>().await.unwrap();
    let updated = t.get("", "topic_partial").await.expect("conversation should exist");
    assert!(!updated.is_partial, "incoming chat should set is_partial to false");
    assert_eq!(updated.last_seq, 1);
    assert_eq!(updated.unread, 1);
}

/// Test that set_conversation_read_local works even on a partial conversation.
#[tokio::test]
async fn test_set_conversation_read_on_partial() {
    let store = ClientStore::new("", ":memory:", "http://test", "token", "receiver-user");

    let mut conv = Conversation::new("topic_partial_read");
    conv.owner_id = "receiver-user".to_string();
    conv.is_partial = true;
    conv.last_seq = 5;
    conv.last_read_seq = 2;
    conv.unread = 3;
    let t = store.message_storage.table::<Conversation>().await.unwrap();
    t.set("", "topic_partial_read", Some(&conv)).await.unwrap();
    drop(t);

    let result = store.set_conversation_read_local("topic_partial_read", "2026-05-21T00:00:00Z", None).await;
    assert!(result.is_some(), "should succeed on partial conversation");

    let updated = result.unwrap();
    assert_eq!(updated.unread, 0);
    assert_eq!(updated.last_read_seq, 5);

    let t = store.message_storage.table::<Conversation>().await.unwrap();
    let persisted = t.get("", "topic_partial_read").await.unwrap();
    assert_eq!(persisted.unread, 0);
    assert_eq!(persisted.last_read_seq, 5);
}
