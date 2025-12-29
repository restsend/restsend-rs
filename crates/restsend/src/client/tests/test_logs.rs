use crate::{
    callback,
    client::{
        tests::{
            test_client::{TestCallbackImpl, TestMessageCakllbackImpl},
            TEST_ENDPOINT,
        },
        Client,
    },
    models::{Content, ContentType, GetChatLogsResult},
    request::ChatRequest,
    services::auth::{login_with_password, signup},
    utils::{check_until, init_log},
};
use log::info;
use std::{
    sync::{
        atomic::{AtomicBool, AtomicI64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

#[tokio::test]
async fn test_client_fetch_logs() {
    signup(
        TEST_ENDPOINT.to_string(),
        "bob1".to_string(),
        "bob:demo".to_string(),
    )
    .await
    .ok();
    signup(
        TEST_ENDPOINT.to_string(),
        "bob2".to_string(),
        "bob:demo".to_string(),
    )
    .await
    .ok();

    let info = login_with_password(
        TEST_ENDPOINT.to_string(),
        "bob1".to_string(),
        "bob:demo".to_string(),
    )
    .await;
    let c = Client::new("".to_string(), "".to_string(), &info.unwrap());
    let topic_id = c
        .create_chat("bob2".to_string())
        .await
        .expect("create chat failed")
        .topic_id;

    let (local_logs, need_fetch) = c.store.get_chat_logs(&topic_id, 0, None, 10).await.unwrap();
    assert_eq!(need_fetch, true);
    assert_eq!(local_logs.items.len(), 0);
    let mut last_seq = 0;
    let send_count = 2;
    for i in 0..send_count {
        let req = ChatRequest::new_text(
            &topic_id,
            &format!("hello from rust unittest bob1->bob2 {}", i),
        );
        let resp = c
            .send_chat_request(topic_id.to_string(), req)
            .await
            .unwrap();
        last_seq = resp.seq;
    }
    struct TestSyncLogsCallbackImpl {
        result: Arc<Mutex<Option<GetChatLogsResult>>>,
    }

    impl callback::SyncChatLogsCallback for TestSyncLogsCallbackImpl {
        fn on_success(&self, r: GetChatLogsResult) {
            let mut result = self.result.lock().unwrap();
            result.replace(r);
        }
        fn on_fail(&self, _reason: crate::Error) {
            panic!("on_fail {:?}", _reason);
        }
    }
    let result = Arc::new(Mutex::new(None));

    let cb = TestSyncLogsCallbackImpl {
        result: result.clone(),
    };

    c.sync_chat_logs_quick(
        topic_id.to_string(),
        None,
        send_count,
        Box::new(cb),
        Some(true),
    )
    .await;

    check_until(Duration::from_secs(3), || result.lock().unwrap().is_some())
        .await
        .unwrap();

    let r = result.lock().unwrap().take().unwrap();
    let (local_logs, need_fetch) = c
        .store
        .get_chat_logs(&topic_id, 0, None, send_count)
        .await
        .unwrap();
    assert_eq!(need_fetch, false);
    assert_eq!(r.start_seq, local_logs.start_sort_value);
    assert_eq!(r.end_seq, local_logs.end_sort_value);
    assert_eq!(last_seq, local_logs.start_sort_value);
}

#[tokio::test]
async fn test_client_recall_log() {
    init_log("INFO".to_string(), true);
    let info = login_with_password(
        TEST_ENDPOINT.to_string(),
        "vitalik".to_string(),
        "vitalik:demo".to_string(),
    )
    .await;

    let c = Client::new("".to_string(), "".to_string(), &info.unwrap());
    let recv_message_count = Arc::new(AtomicI64::new(0));
    c.set_callback(Some(Box::new(TestCallbackImpl {
        last_topic_id: Arc::new(Mutex::new("".to_string())),
        is_connected: Arc::new(AtomicBool::new(false)),
        is_recv_message: Arc::new(AtomicBool::new(false)),
        recv_message_count: recv_message_count.clone(),
        is_update_conversation: Arc::new(AtomicBool::new(false)),
    })));

    c.connect().await;

    check_until(Duration::from_secs(3), || {
        c.connection_status() == "connected"
    })
    .await
    .unwrap();
    let conversation = c.create_chat("guido".to_string()).await.unwrap();
    let topic_id = conversation.topic_id;
    let mut first_send_id = "".to_string();
    let mut last_send_id = "".to_string();

    let send_count = 2;
    for i in 0..send_count {
        let content = Content::new_text(
            ContentType::Text,
            &format!("hello from rust unittest vitalik->guido {}", i),
        );
        let is_ack = Arc::new(AtomicBool::new(false));
        let msg_cb = Box::new(TestMessageCakllbackImpl {
            is_sent: Arc::new(AtomicBool::new(false)),
            is_ack: is_ack.clone(),
            last_error: Arc::new(Mutex::new("".to_string())),
        });
        let chat_id = c
            .do_send(topic_id.to_string(), content, Some(msg_cb))
            .await
            .unwrap();
        if i == 0 {
            first_send_id = chat_id;
        } else {
            last_send_id = chat_id;
        }

        check_until(Duration::from_secs(3), || is_ack.load(Ordering::Relaxed))
            .await
            .unwrap();
    }
    check_until(Duration::from_secs(3), || {
        recv_message_count.load(Ordering::Relaxed) as u32 >= send_count
    })
    .await
    .unwrap();

    info!("do recall first_send_id {}", first_send_id);

    let is_sent = Arc::new(AtomicBool::new(false));
    let is_ack = Arc::new(AtomicBool::new(false));

    let msg_cb = Box::new(TestMessageCakllbackImpl {
        is_sent: is_sent.clone(),
        is_ack: is_ack.clone(),
        last_error: Arc::new(Mutex::new("".to_string())),
    });

    let recall_id = c
        .do_recall(topic_id.to_string(), first_send_id.clone(), Some(msg_cb))
        .await
        .unwrap();

    check_until(Duration::from_secs(3), || is_ack.load(Ordering::Relaxed))
        .await
        .unwrap();

    let (local_logs, need_fetch) = c
        .store
        .get_chat_logs(&topic_id, 0, None, send_count + 1)
        .await
        .unwrap();
    assert_eq!(need_fetch, true);
    assert!(local_logs.items.len() >= send_count as usize);
    assert_eq!(local_logs.items[0].id, recall_id);
    assert_eq!(local_logs.items[1].id, last_send_id);
}

struct TopicGuard {
    topic_id: String,
    client: Arc<Client>,
}

impl Drop for TopicGuard {
    fn drop(&mut self) {
        let topic_id = self.topic_id.clone();
        let client = self.client.clone();
        let f = async move { client.dismiss_topic(topic_id).await.unwrap() };
        tokio::spawn(f);
    }
}

#[tokio::test]
async fn test_client_sync_logs() {
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
    signup(
        TEST_ENDPOINT.to_string(),
        "vivian3".to_string(),
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

    let c = Client::new("".to_string(), "".to_string(), &info);
    let recv_message_count = Arc::new(AtomicI64::new(0));
    c.set_callback(Some(Box::new(TestCallbackImpl {
        last_topic_id: Arc::new(Mutex::new("".to_string())),
        is_connected: Arc::new(AtomicBool::new(false)),
        is_recv_message: Arc::new(AtomicBool::new(false)),
        recv_message_count: recv_message_count.clone(),
        is_update_conversation: Arc::new(AtomicBool::new(false)),
    })));

    c.connect().await;

    check_until(Duration::from_secs(3), || {
        c.connection_status() == "connected"
    })
    .await
    .unwrap();

    let members = vec!["vivian2".to_string(), "vivian3".to_string()];

    let topic = c
        .create_topic(members, None, None, None)
        .await
        .expect("create_topic");
    let topic_id = topic.topic_id;

    let _guard = TopicGuard {
        topic_id: topic_id.clone(),
        client: c.clone(),
    };

    let send_count = 3;
    for i in 0..send_count {
        let content = Content::new_text(
            ContentType::Text,
            &format!("hello from rust unittest vitalik->topic {}", i),
        );
        let is_ack = Arc::new(AtomicBool::new(false));
        let msg_cb = Box::new(TestMessageCakllbackImpl {
            is_sent: Arc::new(AtomicBool::new(false)),
            is_ack: is_ack.clone(),
            last_error: Arc::new(Mutex::new("".to_string())),
        });
        c.do_send(topic_id.to_string(), content, Some(msg_cb))
            .await
            .unwrap();
        check_until(Duration::from_secs(3), || is_ack.load(Ordering::Relaxed))
            .await
            .unwrap();
    }

    struct TestSyncLogsCallbackImpl {
        result: Arc<Mutex<Option<GetChatLogsResult>>>,
    }

    impl callback::SyncChatLogsCallback for TestSyncLogsCallbackImpl {
        fn on_success(&self, r: GetChatLogsResult) {
            let mut result = self.result.lock().unwrap();
            result.replace(r);
        }
        fn on_fail(&self, _reason: crate::Error) {
            panic!("on_fail {:?}", _reason);
        }
    }

    let result = Arc::new(Mutex::new(None));
    let callback = Box::new(TestSyncLogsCallbackImpl {
        result: result.clone(),
    });
    c.sync_chat_logs_quick(topic_id, None, 0, callback, Some(true))
        .await;
    let r = result.lock().unwrap().take().unwrap();
    assert!(!r.has_more);
    assert_eq!(r.items.len(), send_count as usize);
    let mut last_seq = r.items.first().unwrap().seq;
    r.items.iter().for_each(|v| {
        assert!(last_seq >= v.seq);
        last_seq = v.seq;
    });
}
