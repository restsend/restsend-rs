use crate::tests::check_until;
use log::warn;
use std::sync::{Arc, Mutex};
use tokio::time::Duration;

#[test]
fn test_client_websocket_connect_with_handle_incoming() {
    crate::init_log(String::from("debug"), true);
    let c = Arc::new(super::login_with("alice", "alice:demo"));
    let run_c = c.clone();

    struct TestWebsocketCallBack {
        is_fail: Arc<Mutex<bool>>,
        is_done: Arc<Mutex<bool>>,
    }

    impl crate::Callback for TestWebsocketCallBack {
        fn on_topic_message(&self, _topic_id: String, _message: crate::models::ChatLog) -> bool {
            //*self.is_done.lock().unwrap() = true;
            true
        }

        fn on_send_message_fail(&self, topic_id: String, chat_id: String, code: u32) {
            warn!("on_send_message_fail: {} {} {}", topic_id, chat_id, code);
            *self.is_fail.lock().unwrap() = true;
            *self.is_done.lock().unwrap() = true;
        }
    }
    let is_done = Arc::new(Mutex::new(false));
    let is_fail = Arc::new(Mutex::new(false));
    let cb = Box::new(TestWebsocketCallBack {
        is_done: is_done.clone(),
        is_fail: is_fail.clone(),
    });
    c.set_callback(Some(cb));

    std::thread::spawn(move || {
        assert!(run_c.run_loop().is_ok());
    });

    check_until(Duration::from_secs(3), || {
        c.get_network_state() == crate::NetworkState::Connected
    })
    .expect("connect timeout");

    c.do_send_text("not_exist_topic".to_string(), "".to_string(), None, None)
        .expect("send text");

    check_until(Duration::from_secs(3), || *is_done.lock().unwrap()).expect("must send message");
    assert_eq!(*is_fail.lock().unwrap(), true);
}
