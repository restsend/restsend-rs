use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicBool, AtomicI64, AtomicU32},
        Arc, Mutex,
    },
    time::Duration,
};

use crate::{
    callback,
    client::{
        tests::{test_client::TestCallbackImpl, TEST_ENDPOINT},
        Client,
    },
    services::auth::{login_with_password, signup},
    utils::{check_until, init_log},
};

#[tokio::test]
async fn test_sync_conversations() {
    init_log("INFO".to_string(), true);
    signup(
        TEST_ENDPOINT.to_string(),
        "vivian1".to_string(),
        "vivian:demo".to_string(),
    )
    .await
    .ok();

    signup(
        TEST_ENDPOINT.to_string(),
        "vivian2".to_string(),
        "vivian:demo".to_string(),
    )
    .await
    .ok();

    let info = login_with_password(
        TEST_ENDPOINT.to_string(),
        "vivian1".to_string(),
        "vivian:demo".to_string(),
    )
    .await
    .expect("login failed");

    let vivian_1 = Client::new("".to_string(), "".to_string(), &info);

    let vivian1_callback = Box::new(TestCallbackImpl {
        is_connected: Arc::new(AtomicBool::new(false)),
        last_topic_id: Arc::new(Mutex::new("".to_string())),
        is_recv_message: Arc::new(AtomicBool::new(false)),
        recv_message_count: Arc::new(AtomicI64::new(0)),
        is_update_conversation: Arc::new(AtomicBool::new(false)),
    });

    vivian_1.set_callback(Some(vivian1_callback));

    vivian_1.connect().await;
    check_until(Duration::from_secs(3), || {
        vivian_1.connection_status() == "connected"
    })
    .await
    .unwrap();

    struct TestSyncConversationCallbackImpl {
        sync_count: AtomicU32,
    }

    impl callback::SyncConversationsCallback for TestSyncConversationCallbackImpl {
        fn on_success(&self, updated_at: String, last_removed_at: Option<String>, count: u32) {
            log::info!("on_success updated_at: {} last_removed_at: {:?} count: {}", updated_at, last_removed_at, count);
            self.sync_count
                .store(count, std::sync::atomic::Ordering::Relaxed);
        }
        fn on_fail(&self, _reason: crate::Error) {
            panic!("on_fail {:?}", _reason);
        }
    }

    let topic_id = vivian_1
        .create_chat("vivian2".to_string())
        .await
        .unwrap()
        .topic_id;

    let vivian_2 = Client::new("".to_string(), "".to_string(), &info);

    struct TestRemovedCallbackImpl {
        pub removed_topic_ids: Arc<Mutex<HashSet<String>>>,
    }

    impl callback::Callback for TestRemovedCallbackImpl {
        fn on_conversation_removed(&self, conversation_id: String) {
            log::info!("on_conversation_removed: {}", conversation_id);
            self.removed_topic_ids
                .lock()
                .unwrap()
                .insert(conversation_id);
        }
    }

    let removed_topic_ids = Arc::new(Mutex::new(HashSet::new()));
    let vivian2_callback = Box::new(TestRemovedCallbackImpl {
        removed_topic_ids: removed_topic_ids.clone(),
    });

    vivian_2.set_callback(Some(vivian2_callback));

    vivian_2.connect().await;
    check_until(Duration::from_secs(3), || {
        vivian_2.connection_status() == "connected"
    })
    .await
    .unwrap();

    vivian_1.remove_conversation(topic_id.clone()).await;

    log::info!("must removed: {}", topic_id);
    check_until(Duration::from_secs(2), || {
        removed_topic_ids.lock().unwrap().contains(&topic_id)
    })
    .await
    .unwrap();

    let vivian2_callback = Box::new(TestSyncConversationCallbackImpl {
        sync_count: AtomicU32::new(0),
    });

    let removed_topic_ids = Arc::new(Mutex::new(HashSet::new()));
    let vivian_3 = Client::new("".to_string(), "".to_string(), &info);
    let vivian3_callback = Box::new(TestRemovedCallbackImpl {
        removed_topic_ids: removed_topic_ids.clone(),
    });

    vivian_3.set_callback(Some(vivian3_callback));
    vivian_3
        .sync_conversations(None, 0, false, None, None, None, vivian2_callback)
        .await;
    assert!(!removed_topic_ids.lock().unwrap().contains(&topic_id));
}
