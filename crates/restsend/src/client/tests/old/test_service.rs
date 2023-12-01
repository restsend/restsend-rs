#[test]
fn test_client_auth() {
    let alice = super::login_with("alice", "alice:demo");
    _ = alice;
}

#[test]
fn test_single_topic() {
    let alice = super::login_with("alice", "alice:demo");
    let bob = super::login_with("bob", "bob:demo");
    alice
        .set_allow_guest_chat(true)
        .expect("set_allow_guest_chat failed");
    bob.set_allow_guest_chat(true)
        .expect("set_allow_guest_chat failed");

    let topic = alice.create_chat("bob".to_string()).unwrap();
    assert!(topic.id == "alice:bob" && !topic.multiple);
    let topic = bob.create_chat("alice".to_string()).unwrap();
    assert!(topic.id == "bob:alice" && !topic.multiple);

    let lr = alice.get_conversations("".to_string(), 100).unwrap();
    assert!(lr.has_more == false && lr.items.iter().any(|i| i.topic_id == "alice:bob"));
    assert!(alice
        .set_conversation_sticky("alice:bob".to_string(), true)
        .is_ok());
    assert!(alice
        .set_conversation_mute("alice:bob".to_string(), true)
        .is_ok());

    let lr = bob.get_conversations("".to_string(), 100).unwrap();
    assert!(lr.has_more == false && lr.items.iter().any(|i| i.topic_id == "bob:alice"));
    assert!(bob
        .set_conversation_sticky("bob:alice".to_string(), true)
        .is_ok());
    assert!(bob
        .set_conversation_mute("bob:alice".to_string(), true)
        .is_ok());
}

#[test]
fn test_mul_topic() {
    let alice = super::login_with("alice", "alice:demo");
    let bob = super::login_with("bob", "bob:demo");
    let carol = super::login_with("carol", "carol:demo");

    let topic = alice
        .create_topic(
            "multiple_topic".to_string(),
            "icon".to_string(),
            vec!["alice".to_string(), "bob".to_string(), "carol".to_string()],
        )
        .unwrap();
    assert!(topic.name == "multiple_topic" && topic.multiple);
    assert!(alice
        .update_topic_notice(topic.id.clone(), "notice".to_string())
        .is_ok());
    assert!(alice.get_topic(topic.id.clone()).is_ok());
    assert!(bob.get_topic(topic.id.clone()).is_ok());
    assert!(carol.get_topic(topic.id.clone()).is_ok());
    let lr = alice
        .get_topic_members(topic.id.clone(), "".to_string(), 100)
        .unwrap();
    assert!(
        lr.has_more == false
            && lr.items.len() == 3
            && lr.items.iter().any(|m| m.user_id == "alice")
            && lr.items.iter().any(|m| m.user_id == "bob")
            && lr.items.iter().any(|m| m.user_id == "carol")
    );
    assert!(bob.get_topic_owner(topic.id.clone()).unwrap().user_id == "alice");
    assert!(alice.get_topic_owner(topic.id.clone()).unwrap().user_id == "alice");
    assert!(carol.get_topic_owner(topic.id.clone()).unwrap().user_id == "alice");

    let notice = alice.get_topic(topic.id.clone()).unwrap().notice.unwrap();
    assert!(notice.text == "notice" && notice.publisher == "alice");
    let notice = bob.get_topic(topic.id.clone()).unwrap().notice.unwrap();
    assert!(notice.text == "notice" && notice.publisher == "alice");
    let notice = carol.get_topic(topic.id.clone()).unwrap().notice.unwrap();
    assert!(notice.text == "notice" && notice.publisher == "alice");
    assert!(alice
        .silent_topic_member(topic.id.clone(), "carol".to_string(), "".to_string())
        .is_ok());
    assert!(alice.silent_topic(topic.id.clone(), "".to_string()).is_ok());
    assert!(carol.quit_topic(topic.id.clone()).is_ok());

    assert!(carol
        .join_topic(topic.id.clone(), "rejoin".to_string(), "".to_string())
        .is_ok());
    assert!(alice
        .remove_topic_member(topic.id.clone(), "carol".to_string())
        .is_ok());
    assert!(alice.dismiss_topic(topic.id.clone()).is_ok());
    assert!(alice.remove_conversation(topic.id.clone()).is_ok());
    assert!(bob.remove_conversation(topic.id.clone()).is_ok());
    assert!(carol.remove_conversation(topic.id.clone()).is_ok());
}
