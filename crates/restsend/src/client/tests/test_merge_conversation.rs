use crate::{
    client::store::ClientStore,
    models::{ChatLog, Content, Conversation},
};

#[tokio::test]
async fn test_merge_conversation_preserves_server_last_message() {
    let store = ClientStore::new("", ":memory:", "http://test", "token", "user1");

    let mut server_conversation = Conversation::new("test_topic");
    server_conversation.owner_id = "user1".to_string();
    server_conversation.last_seq = 100;
    server_conversation.last_message_seq = Some(100);
    server_conversation.last_message = Some(Content {
        content_type: "text".to_string(),
        text: "Hello from server".to_string(),
        ..Default::default()
    });
    server_conversation.last_message_at = "2026-01-30T10:00:00Z".to_string();
    server_conversation.last_sender_id = "user123".to_string();

    let result = store.update_conversation(server_conversation.clone()).await;

    assert!(result.is_ok());
    let merged = result.unwrap();

    assert!(merged.last_message.is_some());
    assert_eq!(
        merged.last_message.as_ref().unwrap().text,
        "Hello from server"
    );
    assert_eq!(merged.last_message_seq, Some(100));
    assert_eq!(merged.last_sender_id, "user123");

    let mut new_server_conversation = Conversation::new("test_topic");
    new_server_conversation.owner_id = "user1".to_string();
    new_server_conversation.last_seq = 101;
    new_server_conversation.last_message_seq = Some(101);
    new_server_conversation.last_message = Some(Content {
        content_type: "text".to_string(),
        text: "New message from server".to_string(),
        ..Default::default()
    });
    new_server_conversation.last_message_at = "2026-01-30T10:01:00Z".to_string();
    new_server_conversation.last_sender_id = "user456".to_string();

    let result2 = store
        .update_conversation(new_server_conversation.clone())
        .await;

    assert!(result2.is_ok());
    let merged2 = result2.unwrap();

    assert!(merged2.last_message.is_some());
    assert_eq!(
        merged2.last_message.as_ref().unwrap().text,
        "New message from server"
    );
    assert_eq!(merged2.last_message_seq, Some(101));
    assert_eq!(merged2.last_sender_id, "user456");
}

#[tokio::test]
async fn test_merge_conversation_prefers_local_when_newer() {
    let store = ClientStore::new("", ":memory:", "http://test", "token", "user1");

    let chat_log = ChatLog {
        id: "log_100".to_string(),
        topic_id: "test_topic".to_string(),
        seq: 100,
        sender_id: "local_user".to_string(),
        created_at: "2026-01-30T10:02:00Z".to_string(),
        content: Content {
            content_type: "text".to_string(),
            text: "Local message".to_string(),
            ..Default::default()
        },
        ..Default::default()
    };

    let table = store.message_storage.table::<ChatLog>().await.unwrap();
    table
        .set("test_topic", &chat_log.id, Some(&chat_log))
        .await
        .ok();

    let mut server_conversation = Conversation::new("test_topic");
    server_conversation.owner_id = "user1".to_string();
    server_conversation.last_seq = 99;
    server_conversation.last_message_seq = Some(99);
    server_conversation.last_message = Some(Content {
        content_type: "text".to_string(),
        text: "Old server message".to_string(),
        ..Default::default()
    });
    server_conversation.last_message_at = "2026-01-30T10:00:00Z".to_string();
    server_conversation.last_sender_id = "server_user".to_string();

    let conv_table = store.message_storage.table::<Conversation>().await.unwrap();
    conv_table
        .set(
            "",
            &server_conversation.topic_id,
            Some(&server_conversation),
        )
        .await
        .ok();

    let mut new_conversation = Conversation::new("test_topic");
    new_conversation.owner_id = "user1".to_string();
    new_conversation.last_seq = 100;
    new_conversation.last_message_seq = Some(100);
    new_conversation.last_message = Some(Content {
        content_type: "text".to_string(),
        text: "Server message seq 100".to_string(),
        ..Default::default()
    });

    let result = store.update_conversation(new_conversation.clone()).await;

    assert!(result.is_ok());
    let merged = result.unwrap();

    assert!(merged.last_message.is_some());
    assert_eq!(merged.last_message.as_ref().unwrap().text, "Local message");
    assert_eq!(merged.last_message_seq, Some(100));
    assert_eq!(merged.last_sender_id, "local_user");
}

#[tokio::test]
async fn test_merge_conversation_prefers_server_when_local_older() {
    let store = ClientStore::new("", ":memory:", "http://test", "token", "user1");

    let old_chat_log = ChatLog {
        id: "log_98".to_string(),
        topic_id: "test_topic".to_string(),
        seq: 98,
        sender_id: "old_local_user".to_string(),
        created_at: "2026-01-30T09:00:00Z".to_string(),
        content: Content {
            content_type: "text".to_string(),
            text: "Old local message".to_string(),
            ..Default::default()
        },
        ..Default::default()
    };

    let table = store.message_storage.table::<ChatLog>().await.unwrap();
    table
        .set("test_topic", &old_chat_log.id, Some(&old_chat_log))
        .await
        .ok();

    let mut old_conversation = Conversation::new("test_topic");
    old_conversation.owner_id = "user1".to_string();
    old_conversation.last_seq = 98;
    old_conversation.last_message_seq = Some(98);

    let conv_table = store.message_storage.table::<Conversation>().await.unwrap();
    conv_table
        .set("", &old_conversation.topic_id, Some(&old_conversation))
        .await
        .ok();

    let mut new_server_conversation = Conversation::new("test_topic");
    new_server_conversation.owner_id = "user1".to_string();
    new_server_conversation.last_seq = 105;
    new_server_conversation.last_message_seq = Some(105);
    new_server_conversation.last_message = Some(Content {
        content_type: "text".to_string(),
        text: "New server message seq 105".to_string(),
        ..Default::default()
    });
    new_server_conversation.last_message_at = "2026-01-30T10:05:00Z".to_string();
    new_server_conversation.last_sender_id = "new_server_user".to_string();

    let result = store
        .update_conversation(new_server_conversation.clone())
        .await;

    assert!(result.is_ok());
    let merged = result.unwrap();

    assert!(merged.last_message.is_some());
    assert_eq!(
        merged.last_message.as_ref().unwrap().text,
        "New server message seq 105"
    );
    assert_eq!(merged.last_message_seq, Some(105));
    assert_eq!(merged.last_sender_id, "new_server_user");
}
