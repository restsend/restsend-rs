use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc,
};

use crate::{
    callback,
    client::Client,
    models::{Content, Conversation},
};

fn make_conversation(topic_id: &str, updated_at: &str) -> Conversation {
    Conversation {
        topic_id: topic_id.to_string(),
        owner_id: "test-user".to_string(),
        updated_at: updated_at.to_string(),
        last_seq: 0,
        last_message: Some(Content {
            content_type: "text".to_string(),
            text: "hello".to_string(),
            ..Default::default()
        }),
        ..Default::default()
    }
}

struct TestSyncCallback {
    success_called: Arc<AtomicBool>,
    fail_called: Arc<AtomicBool>,
    sync_count: Arc<AtomicU32>,
}

impl callback::SyncConversationsCallback for TestSyncCallback {
    fn on_success(
        &self,
        _updated_at: String,
        _last_removed_at: Option<String>,
        count: u32,
        _total: u32,
    ) {
        self.success_called.store(true, Ordering::Relaxed);
        self.sync_count.store(count, Ordering::Relaxed);
    }
    fn on_fail(&self, _e: crate::Error) {
        self.fail_called.store(true, Ordering::Relaxed);
    }
}

/// Test that sync_max_count limits how many local conversations are loaded
/// in the first loop when category is provided.
#[tokio::test]
async fn test_sync_first_page_respects_sync_max_count() {
    let db_name = "test_sync_max_count";
    let info = crate::models::user::AuthInfo::new(
        "http://127.0.0.1:1",
        "test-sync-limit-user",
        "test-token",
    );
    let client = Client::new("".to_string(), db_name.to_string(), &info);

    // Insert 50 conversations with unique timestamps
    let t = client
        .store
        .message_storage
        .table::<Conversation>()
        .await
        .unwrap();
    for i in 0..50u32 {
        let ts = format!("2026-01-{:02}T{:02}:00:00Z", 1 + (i / 24) as u8, i % 24);
        let conv = make_conversation(&format!("max_topic_{}", i), &ts);
        t.set("", &conv.topic_id, Some(&conv)).await.unwrap();
    }
    drop(t);

    let success_called = Arc::new(AtomicBool::new(false));
    let fail_called = Arc::new(AtomicBool::new(false));
    let sync_count = Arc::new(AtomicU32::new(0));

    client
        .sync_conversations(
            None,
            None,
            Some("test-category".to_string()),
            Some(10),
            5,
            false,
            None,
            None,
            None,
            Box::new(TestSyncCallback {
                success_called: success_called.clone(),
                fail_called: fail_called.clone(),
                sync_count: sync_count.clone(),
            }),
        )
        .await;

    let count = sync_count.load(Ordering::Relaxed);
    // With inclusive bound, each batch after the first has 1 duplicate.
    // limit=5: batch1 adds 5, batch2 adds 4 (1 dup), batch3 adds 4 (1 dup) → total=13
    // sync_max_count=10 check fires after batch3 (13 >= 10).
    assert!(
        count <= 15,
        "expected at most ~13 conversations, got {}",
        count
    );
    assert!(
        count >= 5,
        "expected at least 5, got {}",
        count
    );
}

/// Test that without sync_max_count (0), all local conversations are loaded.
#[tokio::test]
async fn test_sync_first_page_no_limit_loads_all() {
    let db_name = "test_sync_no_limit";
    let info = crate::models::user::AuthInfo::new(
        "http://127.0.0.1:1",
        "test-no-limit-user",
        "test-token",
    );
    let client = Client::new("".to_string(), db_name.to_string(), &info);

    let t = client
        .store
        .message_storage
        .table::<Conversation>()
        .await
        .unwrap();
    for i in 0..10u32 {
        let ts = format!("2026-02-{:02}T{:02}:00:00Z", 1 + (i / 24) as u8, i % 24);
        let conv = make_conversation(&format!("nolimit_topic_{}", i), &ts);
        t.set("", &conv.topic_id, Some(&conv)).await.unwrap();
    }
    drop(t);

    let success_called = Arc::new(AtomicBool::new(false));
    let fail_called = Arc::new(AtomicBool::new(false));
    let sync_count = Arc::new(AtomicU32::new(0));

    client
        .sync_conversations(
            None,
            None,
            Some("test-category".to_string()),
            Some(0),
            5,
            false,
            None,
            None,
            None,
            Box::new(TestSyncCallback {
                success_called: success_called.clone(),
                fail_called: fail_called.clone(),
                sync_count: sync_count.clone(),
            }),
        )
        .await;

    let count = sync_count.load(Ordering::Relaxed);
    // With inclusive bound, each batch after the first has 1 duplicate.
    // 10 items with limit=5: batch1 adds 5, batch2 adds 4 (1 dup), batch3 adds 1 → total=10
    assert_eq!(
        count, 10,
        "expected all 10 conversations loaded, got {}",
        count
    );
}

/// Test that sync_max_count with None (default for syncConversations)
/// still loads all conversations.
#[tokio::test]
async fn test_sync_conversations_no_limit_default() {
    let db_name = "test_sync_default";
    let info = crate::models::user::AuthInfo::new(
        "http://127.0.0.1:1",
        "test-default-user",
        "test-token",
    );
    let client = Client::new("".to_string(), db_name.to_string(), &info);

    let t = client
        .store
        .message_storage
        .table::<Conversation>()
        .await
        .unwrap();
    for i in 0..7u32 {
        let ts = format!("2026-03-{:02}T{:02}:00:00Z", 1 + (i / 24) as u8, i % 24);
        let conv = make_conversation(&format!("default_topic_{}", i), &ts);
        t.set("", &conv.topic_id, Some(&conv)).await.unwrap();
    }
    drop(t);

    let success_called = Arc::new(AtomicBool::new(false));
    let fail_called = Arc::new(AtomicBool::new(false));
    let sync_count = Arc::new(AtomicU32::new(0));

    client
        .sync_conversations(
            None,
            None,
            Some("test".to_string()),
            None,
            3,
            false,
            None,
            None,
            None,
            Box::new(TestSyncCallback {
                success_called: success_called.clone(),
                fail_called: fail_called.clone(),
                sync_count: sync_count.clone(),
            }),
        )
        .await;

    let count = sync_count.load(Ordering::Relaxed);
    // 7 items with limit=3: batch1 adds 3, batch2 adds 2 (1 dup), batch3 adds 2 (1 dup) → total=7
    assert_eq!(
        count, 7,
        "expected all 7 conversations, got {}",
        count
    );
}

/// Test that dedup check prevents infinite loop when many conversations
/// share the same updated_at timestamp (inclusive bound edge case).
#[tokio::test]
async fn test_sync_first_page_dedup_breaks_on_duplicate_page() {
    let db_name = "test_dedup";
    let info = crate::models::user::AuthInfo::new(
        "http://127.0.0.1:1",
        "test-dedup-user",
        "test-token",
    );
    let client = Client::new("".to_string(), db_name.to_string(), &info);

    // Insert 8 conversations all with the SAME timestamp.
    // With limit=3 and inclusive bound, the same items keep being returned.
    let t = client
        .store
        .message_storage
        .table::<Conversation>()
        .await
        .unwrap();
    let same_ts = "2026-04-01T12:00:00Z";
    for i in 0..8u32 {
        let conv = make_conversation(&format!("dup_topic_{}", i), same_ts);
        t.set("", &conv.topic_id, Some(&conv)).await.unwrap();
    }
    drop(t);

    let success_called = Arc::new(AtomicBool::new(false));
    let fail_called = Arc::new(AtomicBool::new(false));
    let sync_count = Arc::new(AtomicU32::new(0));

    client
        .sync_conversations(
            None,
            None,
            Some("test".to_string()),
            Some(0),
            3,
            false,
            None,
            None,
            None,
            Box::new(TestSyncCallback {
                success_called: success_called.clone(),
                fail_called: fail_called.clone(),
                sync_count: sync_count.clone(),
            }),
        )
        .await;

    let count = sync_count.load(Ordering::Relaxed);
    // With dedup check: first batch loads 3 unique items.
    // Second batch returns the same 3 items (same timestamp, inclusive bound).
    // Dedup check detects 0 new unique items → break.
    // So count should be exactly 3.
    assert_eq!(
        count, 3,
        "dedup should stop after first batch (3 items), got {}",
        count
    );
}

/// Test that without category, the first loop is skipped entirely.
#[tokio::test]
async fn test_sync_skips_first_loop_when_no_category() {
    let db_name = "test_no_category";
    let info = crate::models::user::AuthInfo::new(
        "http://127.0.0.1:1",
        "test-no-cat-user",
        "test-token",
    );
    let client = Client::new("".to_string(), db_name.to_string(), &info);

    let t = client
        .store
        .message_storage
        .table::<Conversation>()
        .await
        .unwrap();
    for i in 0..5u32 {
        let ts = format!("2026-05-{:02}T{:02}:00:00Z", 1 + (i / 24) as u8, i % 24);
        let conv = make_conversation(&format!("notopic_{}", i), &ts);
        t.set("", &conv.topic_id, Some(&conv)).await.unwrap();
    }
    drop(t);

    let success_called = Arc::new(AtomicBool::new(false));
    let fail_called = Arc::new(AtomicBool::new(false));
    let sync_count = Arc::new(AtomicU32::new(0));

    client
        .sync_conversations(
            None,
            None,
            None,
            Some(0),
            3,
            false,
            None,
            None,
            None,
            Box::new(TestSyncCallback {
                success_called: success_called.clone(),
                fail_called: fail_called.clone(),
                sync_count: sync_count.clone(),
            }),
        )
        .await;

    let count = sync_count.load(Ordering::Relaxed);
    // Without category, first loop is skipped, so 0 local conversations
    // are loaded. Remote sync fails, so count should be 0.
    assert_eq!(
        count, 0,
        "expected 0 conversations loaded when no category, got {}",
        count
    );
}
