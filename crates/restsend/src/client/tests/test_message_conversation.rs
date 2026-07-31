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

/// Helper: create a ChatRequest with a specific content type.
fn make_incoming_chat_with_type(
    topic_id: &str,
    chat_id: &str,
    seq: i64,
    sender_id: &str,
    content_type: &str,
    text: &str,
) -> ChatRequest {
    ChatRequest {
        req_type: "chat".to_string(),
        chat_id: chat_id.to_string(),
        topic_id: topic_id.to_string(),
        seq,
        attendee: sender_id.to_string(),
        created_at: format!("2026-07-13T14:{:02}:00:00Z", seq),
        content: Some(Content {
            content_type: content_type.to_string(),
            text: text.to_string(),
            unreadable: false,
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Test that a metadata message (update.extra) arriving on a conversation
/// with a stale last_message heals the last_message from local ChatLogs.
///
/// This reproduces the session-8237 bug: the conversation list showed an
/// old guest message ("嗯，已经好多天了") while the chat view had newer
/// messages, because update.extra messages set update_last_message=false
/// and the stale last_message was never healed.
#[tokio::test]
async fn test_metadata_message_heals_stale_last_message() {
    let store =
        ClientStore::new("", ":memory:", "http://test", "token", "agent");
    let callback: Arc<RwLock<Option<Box<dyn callback::RsCallback>>>> =
        Arc::new(RwLock::new(Some(Box::new(TestCallback {
            conv_updated: Arc::new(AtomicU32::new(0)),
        }))));

    // Step 1: Set up a conversation with a STALE last_message (seq=10)
    let mut conv = Conversation::new("topic_heal");
    conv.owner_id = "agent".to_string();
    conv.last_seq = 10;
    conv.last_message_seq = Some(10);
    conv.last_message = Some(Content {
        content_type: "wx.text".to_string(),
        text: "嗯，已经好多天了，什么时候能处理".to_string(),
        unreadable: false,
        ..Default::default()
    });
    conv.last_message_at = "2026-07-10T10:00:00Z".to_string();
    conv.last_sender_id = "guest".to_string();
    conv.is_partial = false;
    let t = store.message_storage.table::<Conversation>().await.unwrap();
    t.set("", "topic_heal", Some(&conv)).await.unwrap();
    drop(t);

    // Step 2: Insert local ChatLogs with NEWER readable messages (seq=11, 12)
    let log_t = store
        .message_storage
        .table::<crate::models::ChatLog>()
        .await
        .unwrap();
    for (seq, text) in [(11i64, "agent reply 1"), (12, "agent reply 2")] {
        let log = crate::models::ChatLog {
            id: format!("log_{}", seq),
            topic_id: "topic_heal".to_string(),
            seq,
            sender_id: "agent".to_string(),
            created_at: format!("2026-07-13T14:{:02}:00:00Z", seq),
            content: Content {
                content_type: "wx.text".to_string(),
                text: text.to_string(),
                unreadable: false,
                ..Default::default()
            },
            status: crate::models::ChatLogStatus::Received,
            ..Default::default()
        };
        log_t.set("topic_heal", &log.id, Some(&log)).await.unwrap();
    }
    drop(log_t);

    // Step 3: Send an update.extra message (metadata, update_last_message=false)
    // In production, update.extra messages are unreadable: true (see log seq=130)
    let update_req = ChatRequest {
        req_type: "chat".to_string(),
        chat_id: "update_1".to_string(),
        topic_id: "topic_heal".to_string(),
        seq: 13,
        attendee: "system".to_string(),
        created_at: "2026-07-13T14:13:00:00Z".to_string(),
        content: Some(Content {
            content_type: "update.extra".to_string(),
            text: "log_12".to_string(),
            unreadable: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    store.process_incoming(update_req, callback.clone()).await;

    // Step 4: Verify last_message was HEALED from local ChatLogs
    let t = store.message_storage.table::<Conversation>().await.unwrap();
    let updated = t.get("", "topic_heal").await.unwrap();
    assert_eq!(
        updated.last_message_seq,
        Some(12),
        "last_message_seq should be healed to the latest readable local log"
    );
    assert_eq!(
        updated.last_message.as_ref().unwrap().text,
        "agent reply 2",
        "last_message should be healed from local ChatLogs, not stuck on stale message"
    );
}

/// Test that merge_conversation_from_chat heals last_message when the
/// incoming message is unreadable but local ChatLogs have newer readable messages.
#[tokio::test]
async fn test_unreadable_message_heals_stale_last_message() {
    let store = ClientStore::new("", ":memory:", "http://test", "token", "agent");
    let callback: Arc<RwLock<Option<Box<dyn callback::RsCallback>>>> =
        Arc::new(RwLock::new(Some(Box::new(TestCallback {
            conv_updated: Arc::new(AtomicU32::new(0)),
        }))));

    // Conversation with stale last_message at seq=5
    let mut conv = Conversation::new("topic_unreadable_heal");
    conv.owner_id = "agent".to_string();
    conv.last_seq = 5;
    conv.last_message_seq = Some(5);
    conv.last_message = Some(Content {
        content_type: "text".to_string(),
        text: "old stale message".to_string(),
        unreadable: false,
        ..Default::default()
    });
    conv.last_message_at = "2026-07-10T10:00:00Z".to_string();
    conv.is_partial = false;
    let t = store.message_storage.table::<Conversation>().await.unwrap();
    t.set("", "topic_unreadable_heal", Some(&conv))
        .await
        .unwrap();
    drop(t);

    // Local ChatLog with a newer readable message at seq=6
    let log_t = store
        .message_storage
        .table::<crate::models::ChatLog>()
        .await
        .unwrap();
    let log = crate::models::ChatLog {
        id: "log_6".to_string(),
        topic_id: "topic_unreadable_heal".to_string(),
        seq: 6,
        sender_id: "agent".to_string(),
        created_at: "2026-07-13T14:06:00Z".to_string(),
        content: Content {
            content_type: "text".to_string(),
            text: "newer local message".to_string(),
            unreadable: false,
            ..Default::default()
        },
        status: crate::models::ChatLogStatus::Received,
        ..Default::default()
    };
    log_t.set("topic_unreadable_heal", "log_6", Some(&log))
        .await
        .unwrap();
    drop(log_t);

    // Incoming unreadable message at seq=7 (advances last_seq but not last_message)
    let req = ChatRequest {
        req_type: "chat".to_string(),
        chat_id: "chat_7".to_string(),
        topic_id: "topic_unreadable_heal".to_string(),
        seq: 7,
        attendee: "system".to_string(),
        created_at: "2026-07-13T14:07:00Z".to_string(),
        content: Some(Content {
            content_type: "update.extra".to_string(),
            text: "".to_string(),
            unreadable: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    store.process_incoming(req, callback.clone()).await;

    // last_message should be healed to seq=6 (the latest readable local log)
    let t = store.message_storage.table::<Conversation>().await.unwrap();
    let updated = t.get("", "topic_unreadable_heal").await.unwrap();
    assert_eq!(
        updated.last_message_seq,
        Some(6),
        "last_message should be healed from local ChatLogs"
    );
    assert_eq!(
        updated.last_message.as_ref().unwrap().text,
        "newer local message",
        "last_message text should be from the healed local log"
    );
}

/// Test that an own-message echo (unreadable=true from server) arriving
/// WITHOUT a local Sending log still updates last_message.
///
/// This reproduces the WASM bug where save_outgoing_chat_log's IndexedDB
/// write was fire-and-forget, so the echo's save_incoming_chat_log couldn't
/// find a local Sending log to preserve unreadable=false. The fix detects
/// attendee == user_id and forces unreadable=false for own messages.
#[tokio::test]
async fn test_own_echo_without_local_log_updates_last_message() {
    let store = ClientStore::new("", ":memory:", "http://test", "token", "agent");
    let callback: Arc<RwLock<Option<Box<dyn callback::RsCallback>>>> =
        Arc::new(RwLock::new(Some(Box::new(TestCallback {
            conv_updated: Arc::new(AtomicU32::new(0)),
        }))));

    let mut conv = Conversation::new("topic_own_echo");
    conv.owner_id = "agent".to_string();
    conv.last_seq = 0;
    let t = store.message_storage.table::<Conversation>().await.unwrap();
    t.set("", "topic_own_echo", Some(&conv))
        .await
        .unwrap();
    drop(t);

    // NO local Sending log stored (simulating save_outgoing_chat_log failure
    // or HTTP send path or WASM fire-and-forget race).
    //
    // Server echoes own message with unreadable=true.
    let echo = ChatRequest {
        req_type: "chat".to_string(),
        chat_id: "chat_echo_1".to_string(),
        topic_id: "topic_own_echo".to_string(),
        seq: 1,
        attendee: "agent".to_string(),
        created_at: "2026-07-29T10:00:00Z".to_string(),
        content: Some(Content {
            content_type: "text".to_string(),
            text: "My own message".to_string(),
            unreadable: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    store.process_incoming(echo, callback.clone()).await;

    let t = store.message_storage.table::<Conversation>().await.unwrap();
    let updated = t.get("", "topic_own_echo").await.unwrap();
    assert_eq!(updated.last_seq, 1, "last_seq should advance");
    assert_eq!(
        updated.last_message_seq,
        Some(1),
        "last_message_seq should be set for own echo even without local log"
    );
    assert_eq!(
        updated.last_message.as_ref().unwrap().text,
        "My own message",
        "last_message should contain the echo text"
    );
    assert!(
        !updated.last_message.as_ref().unwrap().unreadable,
        "last_message should be readable for own messages"
    );
}

/// Test that multiple own-message echoes (without local Sending logs) all
/// update last_message correctly, ending with the last message.
#[tokio::test]
async fn test_multiple_own_echoes_update_last_message() {
    let store = ClientStore::new("", ":memory:", "http://test", "token", "agent");
    let callback: Arc<RwLock<Option<Box<dyn callback::RsCallback>>>> =
        Arc::new(RwLock::new(Some(Box::new(TestCallback {
            conv_updated: Arc::new(AtomicU32::new(0)),
        }))));

    let mut conv = Conversation::new("topic_multi_echo");
    conv.owner_id = "agent".to_string();
    conv.last_seq = 0;
    let t = store
        .message_storage
        .table::<Conversation>()
        .await
        .unwrap();
    t.set("", "topic_multi_echo", Some(&conv))
        .await
        .unwrap();
    drop(t);

    for (seq, text) in [(1i64, "First"), (2, "Second"), (3, "Third")] {
        let echo = ChatRequest {
            req_type: "chat".to_string(),
            chat_id: format!("chat_multi_{}", seq),
            topic_id: "topic_multi_echo".to_string(),
            seq,
            attendee: "agent".to_string(),
            created_at: format!("2026-07-29T10:0{}:00Z", seq),
            content: Some(Content {
                content_type: "text".to_string(),
                text: text.to_string(),
                unreadable: true,
                ..Default::default()
            }),
            ..Default::default()
        };
        store.process_incoming(echo, callback.clone()).await;
    }

    let t = store
        .message_storage
        .table::<Conversation>()
        .await
        .unwrap();
    let updated = t.get("", "topic_multi_echo").await.unwrap();
    assert_eq!(updated.last_seq, 3, "last_seq should be 3");
    assert_eq!(
        updated.last_message_seq,
        Some(3),
        "last_message_seq should be 3"
    );
    assert_eq!(
        updated.last_message.as_ref().unwrap().text,
        "Third",
        "last_message should be the latest message"
    );
}

/// Test that the heal function can find a readable message beyond the
/// old 10-log search window when many unreadable messages precede it.
#[tokio::test]
async fn test_heal_searches_beyond_10_unreadable_logs() {
    let store = ClientStore::new("", ":memory:", "http://test", "token", "agent");
    let callback: Arc<RwLock<Option<Box<dyn callback::RsCallback>>>> =
        Arc::new(RwLock::new(Some(Box::new(TestCallback {
            conv_updated: Arc::new(AtomicU32::new(0)),
        }))));

    let mut conv = Conversation::new("topic_heal_50");
    conv.owner_id = "agent".to_string();
    conv.last_seq = 0;
    conv.last_message_seq = Some(0);
    conv.last_message = Some(Content {
        content_type: "text".to_string(),
        text: "old readable".to_string(),
        unreadable: false,
        ..Default::default()
    });
    conv.last_message_at = "2026-07-29T00:00:00Z".to_string();
    let t = store
        .message_storage
        .table::<Conversation>()
        .await
        .unwrap();
    t.set("", "topic_heal_50", Some(&conv))
        .await
        .unwrap();
    drop(t);

    // Insert 15 unreadable ChatLogs (seq 1..15) followed by a readable one at seq=16
    let log_t = store
        .message_storage
        .table::<crate::models::ChatLog>()
        .await
        .unwrap();
    for seq in 1..=15u32 {
        let log = crate::models::ChatLog {
            id: format!("log_u_{}", seq),
            topic_id: "topic_heal_50".to_string(),
            seq: seq as i64,
            sender_id: "system".to_string(),
            created_at: format!("2026-07-29T10:{:02}:00Z", seq),
            content: Content {
                content_type: "update.extra".to_string(),
                text: format!("unreadable_{}", seq),
                unreadable: true,
                ..Default::default()
            },
            status: crate::models::ChatLogStatus::Received,
            ..Default::default()
        };
        log_t.set("topic_heal_50", &log.id, Some(&log))
            .await
            .unwrap();
    }
    let readable_log = crate::models::ChatLog {
        id: "log_r_16".to_string(),
        topic_id: "topic_heal_50".to_string(),
        seq: 16,
        sender_id: "agent".to_string(),
        created_at: "2026-07-29T10:16:00Z".to_string(),
        content: Content {
            content_type: "text".to_string(),
            text: "readable at 16".to_string(),
            unreadable: false,
            ..Default::default()
        },
        status: crate::models::ChatLogStatus::Received,
        ..Default::default()
    };
    log_t.set("topic_heal_50", "log_r_16", Some(&readable_log))
        .await
        .unwrap();
    drop(log_t);

    // Now an unreadable message arrives at seq=17, triggering heal.
    // With the old limit=10, heal could only search back to seq=8, all unreadable.
    // With limit=50, heal can reach seq=16 (readable).
    let req = ChatRequest {
        req_type: "chat".to_string(),
        chat_id: "chat_17".to_string(),
        topic_id: "topic_heal_50".to_string(),
        seq: 17,
        attendee: "system".to_string(),
        created_at: "2026-07-29T10:17:00Z".to_string(),
        content: Some(Content {
            content_type: "update.extra".to_string(),
            text: "trigger".to_string(),
            unreadable: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    store.process_incoming(req, callback.clone()).await;

    let t = store
        .message_storage
        .table::<Conversation>()
        .await
        .unwrap();
    let updated = t.get("", "topic_heal_50").await.unwrap();
    assert_eq!(
        updated.last_message_seq,
        Some(16),
        "heal should find readable message at seq=16 beyond old 10-log window"
    );
    assert_eq!(
        updated.last_message.as_ref().unwrap().text,
        "readable at 16",
        "last_message should be healed from the readable log"
    );
}

/// Test that a genuinely unreadable incoming message from another user
/// does NOT override the last_message (keeps the old readable one).
/// This verifies the fix doesn't break the intended behavior.
#[tokio::test]
async fn test_genuinely_unreadable_keeps_old_last_message() {
    let store = ClientStore::new("", ":memory:", "http://test", "token", "agent");
    let callback: Arc<RwLock<Option<Box<dyn callback::RsCallback>>>> =
        Arc::new(RwLock::new(Some(Box::new(TestCallback {
            conv_updated: Arc::new(AtomicU32::new(0)),
        }))));

    let mut conv = Conversation::new("topic_genuinely_unreadable");
    conv.owner_id = "agent".to_string();
    conv.last_seq = 5;
    conv.last_message_seq = Some(5);
    conv.last_message = Some(Content {
        content_type: "text".to_string(),
        text: "old readable message".to_string(),
        unreadable: false,
        ..Default::default()
    });
    conv.last_message_at = "2026-07-29T09:00:00Z".to_string();
    conv.last_sender_id = "someone".to_string();
    let t = store
        .message_storage
        .table::<Conversation>()
        .await
        .unwrap();
    t.set("", "topic_genuinely_unreadable", Some(&conv))
        .await
        .unwrap();
    drop(t);

    // Genuinely unreadable message from ANOTHER user (not self)
    let req = ChatRequest {
        req_type: "chat".to_string(),
        chat_id: "chat_unreadable_6".to_string(),
        topic_id: "topic_genuinely_unreadable".to_string(),
        seq: 6,
        attendee: "other_user".to_string(),
        created_at: "2026-07-29T10:00:00Z".to_string(),
        content: Some(Content {
            content_type: "text".to_string(),
            text: "hidden content".to_string(),
            unreadable: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    store.process_incoming(req, callback.clone()).await;

    let t = store
        .message_storage
        .table::<Conversation>()
        .await
        .unwrap();
    let updated = t
        .get("", "topic_genuinely_unreadable")
        .await
        .unwrap();
    assert_eq!(updated.last_seq, 6, "last_seq should advance");
    assert_eq!(
        updated.last_message_seq,
        Some(5),
        "last_message_seq should stay at 5 for genuinely unreadable message"
    );
    assert_eq!(
        updated.last_message.as_ref().unwrap().text,
        "old readable message",
        "last_message should keep the old readable content"
    );
}

/// Test 1 (core): own message echo with unreadable=true (server echo, no local
/// Sending log, e.g. HTTP send path) still updates last_message.
///
/// This is the original lastMessage bug. save_incoming_chat_log persists the log
/// via fire-and-forget IndexedDB put, so a subsequent IndexedDB read could miss
/// it. The fix passes the just-saved ChatLog in-memory to merge_conversation_from_chat.
#[tokio::test]
async fn test_own_echo_inmemory_saved_log_updates_last_message() {
    let store = ClientStore::new("", ":memory:", "http://test", "token", "agent");
    let callback: Arc<RwLock<Option<Box<dyn callback::RsCallback>>>> =
        Arc::new(RwLock::new(Some(Box::new(TestCallback {
            conv_updated: Arc::new(AtomicU32::new(0)),
        }))));

    let mut conv = Conversation::new("topic_inmem_echo");
    conv.owner_id = "agent".to_string();
    conv.last_seq = 0;
    let t = store.message_storage.table::<Conversation>().await.unwrap();
    t.set("", "topic_inmem_echo", Some(&conv)).await.unwrap();
    drop(t);

    // No local Sending log (simulating HTTP send path / save_outgoing_chat_log failure).
    let echo = ChatRequest {
        req_type: "chat".to_string(),
        chat_id: "chat_echo_1".to_string(),
        topic_id: "topic_inmem_echo".to_string(),
        seq: 1,
        attendee: "agent".to_string(),
        created_at: "2026-07-31T10:00:00Z".to_string(),
        content: Some(Content {
            content_type: "text".to_string(),
            text: "My reply".to_string(),
            unreadable: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    store.process_incoming(echo, callback.clone()).await;

    let t = store.message_storage.table::<Conversation>().await.unwrap();
    let updated = t.get("", "topic_inmem_echo").await.unwrap();
    assert_eq!(updated.last_seq, 1, "last_seq should advance");
    assert_eq!(
        updated.last_message_seq,
        Some(1),
        "last_message_seq should be set for own echo"
    );
    assert_eq!(
        updated.last_message.as_ref().unwrap().text,
        "My reply",
        "last_message should contain the echo text"
    );
    assert!(
        !updated.last_message.as_ref().unwrap().unreadable,
        "last_message should be readable for own messages"
    );
}

/// Test 2: processing a message echo must NOT clobber conversation.extra.
///
/// This guards against the extra corruption introduced by synchronous IndexedDB
/// puts (which yielded mid-processing and let another task overwrite extra).
#[tokio::test]
async fn test_own_echo_preserves_extra() {
    let store = ClientStore::new("", ":memory:", "http://test", "token", "agent");
    let callback: Arc<RwLock<Option<Box<dyn callback::RsCallback>>>> =
        Arc::new(RwLock::new(Some(Box::new(TestCallback {
            conv_updated: Arc::new(AtomicU32::new(0)),
        }))));

    let mut conv = Conversation::new("topic_extra_preserve");
    conv.owner_id = "agent".to_string();
    conv.last_seq = 0;
    let mut extra = std::collections::HashMap::new();
    extra.insert("isReplay".to_string(), "Y".to_string());
    conv.extra = Some(extra);
    let t = store.message_storage.table::<Conversation>().await.unwrap();
    t.set("", "topic_extra_preserve", Some(&conv)).await.unwrap();
    drop(t);

    let echo = ChatRequest {
        req_type: "chat".to_string(),
        chat_id: "chat_1".to_string(),
        topic_id: "topic_extra_preserve".to_string(),
        seq: 1,
        attendee: "agent".to_string(),
        created_at: "2026-07-31T10:00:00Z".to_string(),
        content: Some(Content {
            content_type: "text".to_string(),
            text: "Reply".to_string(),
            unreadable: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    store.process_incoming(echo, callback.clone()).await;

    let t = store.message_storage.table::<Conversation>().await.unwrap();
    let updated = t.get("", "topic_extra_preserve").await.unwrap();
    assert_eq!(
        updated.extra.as_ref().and_then(|e| e.get("isReplay")),
        Some(&"Y".to_string()),
        "extra must be preserved after processing own echo"
    );
}

/// Test 3: a conversation.update WS message (with extra) updates conversation.extra.
///
/// This is how the "未回复" label is cleared: xwork sets extra.isReplay="N"
/// on the server, which broadcasts a conversation.update to the client.
#[tokio::test]
async fn test_conversation_update_sets_extra() {
    let store = ClientStore::new("", ":memory:", "http://test", "token", "agent");
    let callback: Arc<RwLock<Option<Box<dyn callback::RsCallback>>>> =
        Arc::new(RwLock::new(Some(Box::new(TestCallback {
            conv_updated: Arc::new(AtomicU32::new(0)),
        }))));

    let mut conv = Conversation::new("topic_update_extra");
    conv.owner_id = "agent".to_string();
    conv.extra = Some(std::collections::HashMap::from([(
        "isReplay".to_string(),
        "Y".to_string(),
    )]));
    let t = store.message_storage.table::<Conversation>().await.unwrap();
    t.set("", "topic_update_extra", Some(&conv)).await.unwrap();
    drop(t);

    let update_req = ChatRequest {
        req_type: "chat".to_string(),
        topic_id: "topic_update_extra".to_string(),
        attendee: "agent".to_string(),
        created_at: "2026-07-31T10:00:00Z".to_string(),
        content: Some(Content {
            content_type: "conversation.update".to_string(),
            text: r#"{"extra":{"isReplay":"N"}}"#.to_string(),
            unreadable: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    store.process_incoming(update_req, callback.clone()).await;

    let t = store.message_storage.table::<Conversation>().await.unwrap();
    let updated = t.get("", "topic_update_extra").await.unwrap();
    assert_eq!(
        updated.extra.as_ref().and_then(|e| e.get("isReplay")),
        Some(&"N".to_string()),
        "ConversationUpdate should set extra.isReplay to N"
    );
}

/// Test 4 (race): ConversationUpdate sets extra.isReplay="N", then an own
/// message echo arrives. The echo must NOT revert extra back to "Y".
///
/// This is the exact ordering that triggered the extra corruption race: the
/// echo's merge_conversation_from_chat reads/writes the conversation; with
/// fire-and-forget puts (no yield) it cannot clobber the "N" written by the
/// ConversationUpdate.
#[tokio::test]
async fn test_update_then_echo_preserves_extra() {
    let store = ClientStore::new("", ":memory:", "http://test", "token", "agent");
    let callback: Arc<RwLock<Option<Box<dyn callback::RsCallback>>>> =
        Arc::new(RwLock::new(Some(Box::new(TestCallback {
            conv_updated: Arc::new(AtomicU32::new(0)),
        }))));

    let mut conv = Conversation::new("topic_update_then_echo");
    conv.owner_id = "agent".to_string();
    conv.last_seq = 0;
    conv.extra = Some(std::collections::HashMap::from([(
        "isReplay".to_string(),
        "Y".to_string(),
    )]));
    let t = store.message_storage.table::<Conversation>().await.unwrap();
    t.set("", "topic_update_then_echo", Some(&conv))
        .await
        .unwrap();
    drop(t);

    // Step 1: conversation.update sets isReplay=N
    let update_req = ChatRequest {
        req_type: "chat".to_string(),
        topic_id: "topic_update_then_echo".to_string(),
        attendee: "agent".to_string(),
        created_at: "2026-07-31T10:00:00Z".to_string(),
        content: Some(Content {
            content_type: "conversation.update".to_string(),
            text: r#"{"extra":{"isReplay":"N"}}"#.to_string(),
            unreadable: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    store.process_incoming(update_req, callback.clone()).await;

    // Step 2: own message echo arrives right after
    let echo = ChatRequest {
        req_type: "chat".to_string(),
        chat_id: "chat_1".to_string(),
        topic_id: "topic_update_then_echo".to_string(),
        seq: 1,
        attendee: "agent".to_string(),
        created_at: "2026-07-31T10:00:01Z".to_string(),
        content: Some(Content {
            content_type: "text".to_string(),
            text: "Agent reply".to_string(),
            unreadable: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    store.process_incoming(echo, callback.clone()).await;

    let t = store.message_storage.table::<Conversation>().await.unwrap();
    let updated = t.get("", "topic_update_then_echo").await.unwrap();
    assert_eq!(
        updated.extra.as_ref().and_then(|e| e.get("isReplay")),
        Some(&"N".to_string()),
        "extra must remain N after echo (no race condition)"
    );
    assert_eq!(
        updated.last_message.as_ref().unwrap().text,
        "Agent reply",
        "last_message should be updated by the echo"
    );
    assert!(
        !updated.last_message.as_ref().unwrap().unreadable,
        "echo last_message should be readable"
    );
}
