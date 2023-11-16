use std::sync::{Arc, Mutex};

use crate::{client::Client, models::Conversation, Callback, ChatLog, User};
use log::{info, warn};
use tokio::time::Duration;

/* test single chat
| Event                 | Alice               | Bob                 |
|-----------------------|---------------------|---------------------|
| on_conn               | send_hello          |                     |
| on_msg(alice_hello)   |                     | send_hello          |
| on_msg(bob_hello)     | recall_hello        |                     |
*/

/* test multi chat
| Event                 | Alice               | Bob                 | Carol               |
|-----------------------|---------------------|---------------------|---------------------|
| on_conn               | send_hello          |                     |                     |
| on_msg(alice_hello)   |                     | send_hello          |                     |
| on_msg(bob_hello)     | recall_hello        |                     | send_hello          |
| on_msg(carol_hello)   | silent_carol        |                     |                     |
| on_silent_carol       |                     |                     | quit_topic          |
| on_member(carol_quit) | silent_topic        |                     | rejoin_topic        |
| on_member(carol_join) | kickoff_carol       |                     |                     |
| on_kickoff(carol)     | dismiss_topic       |                     | remove_conversation |
| on_dismiss            | remove_conversation | remove_conversation |                     |
*/
struct TestChatCallBack<'a> {
    user_id: String,
    user: &'a Client,
    is_multi_chat: bool,
    create_topic: bool,
}

impl Callback for TestChatCallBack<'_> {
    fn on_connected(&self) {
        info!("{} received: on_connected", self.user_id);
        if !self.create_topic {
            return;
        }

        let topic = if self.is_multi_chat {
            self.user
                .create_topic(
                    "multiple_topic".to_string(),
                    "icon".to_string(),
                    vec!["alice".to_string(), "bob".to_string(), "carol".to_string()],
                )
                .expect("create topic")
        } else {
            self.user
                .create_chat("bob".to_string())
                .expect("create_chat")
        };
        self.user
            .do_send_text(topic.id, format!("hello from {}", self.user_id), None, None)
            .expect("send text");
    }

    fn on_topic_message(&self, topic_id: String, message: ChatLog) {
        info!(
            "{} received: on_topic_message: {} {:?}",
            self.user_id, topic_id, message
        );
        if message.content.r#type == "text" {
            match message.content.text.as_str() {
                "hello from bob" => {
                    info!("recall hello from {}", self.user_id);
                    let recall_chat_id = self
                        .user
                        .search_chat_log(
                            topic_id.clone(),
                            self.user_id.clone(),
                            format!("hello from {}", self.user_id),
                        )
                        .unwrap()[0]
                        .id
                        .clone();
                    let r = self.user.do_recall(topic_id, recall_chat_id);
                    info!("do recall result: {:?}", r);
                }
                "hello from carol" => {
                    info!("silent carol");
                    assert!(self
                        .user
                        .silent_topic_member(topic_id, "carol".to_string(), "".to_string())
                        .is_ok());
                }
                _ => {}
            }
        }
    }

    fn on_topic_member_updated(&self, topic_id: String, member: User, is_add: bool) {
        info!(
            "{} received: on_topic_member_updated: {} {:?} {}",
            self.user_id, topic_id, member, is_add
        );
        if !is_add {
            assert!(self.user.silent_topic(topic_id, "".to_string()).is_ok());
        } else {
            assert!(self
                .user
                .remove_topic_member(topic_id, "carol".to_string())
                .is_ok());
        }
    }

    fn on_conversation_updated(&self, conversations: Vec<Conversation>) {
        info!(
            "{} received: on_conversation_updated: {:?}",
            self.user_id, conversations
        );
        let last_message = conversations[0].last_message.clone().unwrap();
        if last_message.r#type == "text" {
            if self.is_multi_chat {
                assert!(
                    last_message.text == "hello from alice"
                        || last_message.text == "hello from bob"
                        || last_message.text == "hello from carol"
                );
            } else {
                assert!(
                    last_message.text == "hello from alice"
                        || last_message.text == "hello from bob"
                );
            }
        }
    }

    fn on_topic_kickoff(&self, topic_id: String, admin_id: String, user_id: String) {
        info!(
            "{} received: on_topic_kickoff: {} {} {}",
            self.user_id, topic_id, admin_id, user_id
        );
        assert!(self.user.dismiss_topic(topic_id).is_ok());
    }
    fn on_topic_dismissed(&self, topic_id: String, user_id: String) {
        info!(
            "{} received: on_topic_dismissed: {} {}",
            self.user_id, topic_id, user_id
        );
        assert!(self.user.remove_conversation(topic_id).is_ok());
    }
}

#[test]
fn test_get_conversations() {
    crate::init_log(String::from("debug"), true);
    let alice = Arc::new(super::login_with("alice", "alice:demo"));
    struct TestConversationCallback {
        user_id: String,
        conversation_count: Arc<Mutex<usize>>,
    }

    impl crate::Callback for TestConversationCallback {
        fn on_conversation_updated(&self, _conversations: Vec<Conversation>) {
            info!(
                "{} -> on_conversation_updated: count: {:?}",
                self.user_id,
                _conversations.len()
            );
            *self.conversation_count.lock().unwrap() += _conversations.len();
        }
    }
    let cc = Arc::new(Mutex::new(0));
    let cb = TestConversationCallback {
        user_id: "alice".to_string(),
        conversation_count: cc.clone(),
    };
    alice.set_callback(Some(Box::new(cb)));

    alice.sync_conversations(false).expect("sync conversations");
    assert!(cc.lock().unwrap().clone() > 0);
}
#[test]
fn test_single_chat() {
    crate::init_log(String::from("debug"), true);

    let alice = Arc::new(super::login_with("alice", "alice:demo"));
    let bob = Arc::new(super::login_with("bob", "bob:demo"));

    struct TestSingleChatCallback {
        user_id: String,
        is_fail: Arc<Mutex<bool>>,
        is_done: Arc<Mutex<bool>>,
    }

    impl crate::Callback for TestSingleChatCallback {
        fn on_topic_message(&self, _topic_id: String, _message: crate::models::ChatLog) {
            warn!(
                "on_topic_message: {} -> {} {}",
                self.user_id, _topic_id, _message.id
            );
            *self.is_done.lock().unwrap() = true;
        }

        fn on_send_message_fail(&self, topic_id: String, chat_id: String, code: u32) {
            warn!(
                "on_send_message_fail: {} -> {} {} {}",
                self.user_id, topic_id, chat_id, code
            );
            *self.is_fail.lock().unwrap() = true;
            *self.is_done.lock().unwrap() = true;
        }
    }

    let alice_is_done = Arc::new(Mutex::new(false));
    let alice_is_fail = Arc::new(Mutex::new(false));
    alice.set_callback(Some(Box::new(TestSingleChatCallback {
        user_id: "alice".to_string(),
        is_done: alice_is_done.clone(),
        is_fail: alice_is_fail.clone(),
    })));

    let bob_is_done = Arc::new(Mutex::new(false));
    let bob_is_fail = Arc::new(Mutex::new(false));
    bob.set_callback(Some(Box::new(TestSingleChatCallback {
        user_id: "bob".to_string(),
        is_done: bob_is_done.clone(),
        is_fail: bob_is_fail.clone(),
    })));

    let alice_c = alice.clone();
    std::thread::spawn(move || {
        alice_c.run_loop().unwrap();
    });

    let bob_c = bob.clone();
    std::thread::spawn(move || {
        bob_c.run_loop().unwrap();
    });

    let bob_topic = alice
        .create_chat("bob".to_string())
        .expect("create single chat");

    alice
        .do_send_text(bob_topic.id, "hello from alice".to_string(), None, None)
        .expect("do send text");

    super::check_until(Duration::from_secs(3), || {
        *alice_is_done.lock().unwrap() && *bob_is_done.lock().unwrap()
    })
    .expect("send or recv timeout");

    assert_eq!(*bob_is_fail.lock().unwrap(), false);
    assert_eq!(*alice_is_fail.lock().unwrap(), false);
}

#[test]
fn test_multi_chat() {}

#[test]
fn test_conversation_state() {
    
}
