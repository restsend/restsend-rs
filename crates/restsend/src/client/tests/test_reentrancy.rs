use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc,
};

use crate::{
    callback,
    client::Client,
    models::user::AuthInfo,
};

#[tokio::test]
async fn test_sync_conversations_reentrancy_guard() {
    let info = AuthInfo::new("http://127.0.0.1:1", "test-user", "test-token");

    let client = Client::new("".to_string(), ":memory:".to_string(), &info);

    assert!(!client.store.syncing_conversations.load(Ordering::Relaxed));

    client.store.syncing_conversations.store(true, Ordering::Relaxed);

    struct ReentrancyTestCallback {
        success_called: Arc<AtomicBool>,
        fail_called: Arc<AtomicBool>,
        count: Arc<AtomicU32>,
    }

    impl callback::SyncConversationsCallback for ReentrancyTestCallback {
        fn on_success(
            &self,
            _updated_at: String,
            _last_removed_at: Option<String>,
            count: u32,
            _total: u32,
        ) {
            self.success_called.store(true, Ordering::Relaxed);
            self.count.store(count, Ordering::Relaxed);
        }
        fn on_fail(&self, _e: crate::Error) {
            self.fail_called.store(true, Ordering::Relaxed);
        }
    }

    let success_called = Arc::new(AtomicBool::new(false));
    let fail_called = Arc::new(AtomicBool::new(false));
    let count = Arc::new(AtomicU32::new(u32::MAX));

    client
        .sync_conversations(
            None,
            None,
            None,
            None,
            0,
            false,
            None,
            None,
            None,
            Box::new(ReentrancyTestCallback {
                success_called: success_called.clone(),
                fail_called: fail_called.clone(),
                count: count.clone(),
            }),
        )
        .await;

    assert!(
        success_called.load(Ordering::Relaxed),
        "on_success should be called when sync is skipped"
    );
    assert!(
        !fail_called.load(Ordering::Relaxed),
        "on_fail should NOT be called"
    );
    assert_eq!(
        count.load(Ordering::Relaxed),
        0,
        "count should be 0 when skipped"
    );

    assert!(
        client.store.syncing_conversations.load(Ordering::Relaxed),
        "flag should still be true (held by the simulated in-progress sync)"
    );

    client
        .store
        .syncing_conversations
        .store(false, Ordering::Release);
}
