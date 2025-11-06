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
        fn on_success(
            &self,
            updated_at: String,
            last_removed_at: Option<String>,
            count: u32,
            total: u32,
        ) {
            log::info!(
                "on_success updated_at: {} last_removed_at: {:?} count: {count} total: {total}",
                updated_at,
                last_removed_at,
            );
            self.sync_count
                .store(count, std::sync::atomic::Ordering::Relaxed);
        }
        fn on_fail(&self, reason: crate::Error) {
            panic!("on_fail {:?}", reason);
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
    struct TestCountableCallbackImpl {}

    impl callback::RsCallback for TestRemovedCallbackImpl {
        fn on_conversation_removed(&self, conversation_id: String) {
            log::info!("on_conversation_removed: {}", conversation_id);
            self.removed_topic_ids
                .lock()
                .unwrap()
                .insert(conversation_id);
        }
    }

    impl callback::CountableCallback for TestCountableCallbackImpl {
        fn is_countable(&self, content: crate::models::Content) -> bool {
            content.content_type == "text"
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

    let vivian3_countable_callback = Box::new(TestCountableCallbackImpl {});

    vivian_3.set_callback(Some(vivian3_callback));
    vivian_3.set_countable_callback(Some(vivian3_countable_callback));
    vivian_3
        .sync_conversations(
            None,
            None,
            None,
            None,
            0,
            true,
            None,
            None,
            None,
            vivian2_callback,
        )
        .await;
    assert!(!removed_topic_ids.lock().unwrap().contains(&topic_id));

    let unread_count = vivian_3.get_unread_count().await;
    assert!(unread_count > 0);
}
