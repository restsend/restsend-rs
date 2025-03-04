use crate::{
    callback,
    client::{
        tests::{test_client::TestMessageCakllbackImpl, TEST_ENDPOINT},
        Client,
    },
    models::Attachment,
    request::ChatRequest,
    services::auth::login_with_password,
    utils::check_until,
    utils::init_log,
};
use log::warn;
use std::{
    io::Write,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
    vec,
};
use tempfile::NamedTempFile;

pub(super) struct TestUploadCallbackImpl {}

impl callback::RsCallback for TestUploadCallbackImpl {
    fn on_connected(&self) {
        warn!("on_connected");
    }
    fn on_topic_read(&self, topic_id: String, message: ChatRequest) {
        warn!("on_topic_read: topic_id:{} message:{:?}", topic_id, message);
    }
}
#[tokio::test]
async fn test_client_upload() {
    init_log("INFO".to_string(), true);
    let info = login_with_password(
        TEST_ENDPOINT.to_string(),
        "bob".to_string(),
        "bob:demo".to_string(),
    )
    .await;
    assert!(info.is_ok());

    let c = Client::new("".to_string(), "".to_string(), &info.unwrap());
    let callback = Box::new(TestUploadCallbackImpl {});

    c.set_callback(Some(callback));
    c.connect().await;

    check_until(Duration::from_secs(3), || {
        c.connection_status() == "connected"
    })
    .await
    .expect("connect failed");

    // create a mock png file
    let file_name = "test.png";
    let f = NamedTempFile::new().expect("create temp file failed");
    let mut file = std::fs::File::create(f.path()).expect("create file failed");
    let file_data = "PNG".as_bytes();
    file.write_all(file_data).expect("write file failed");
    for _ in 0..100 {
        let buf = vec![0; 1024];
        file.write_all(&buf).expect("write file failed");
    }
    file.sync_all().expect("sync file failed");
    let file_path = f.path().to_str().unwrap().to_string();

    let attachment = Attachment::from_local(file_name, &file_path, false);

    let is_sent = Arc::new(AtomicBool::new(false));
    let is_ack = Arc::new(AtomicBool::new(false));

    let msg_cb = Box::new(TestMessageCakllbackImpl {
        is_sent: is_sent.clone(),
        is_ack: is_ack.clone(),
        last_error: Arc::new(Mutex::new("".to_string())),
    });

    let r = c
        .do_send_image(
            "bob:alice".to_string(),
            attachment,
            None,
            None,
            Some(msg_cb),
        )
        .await;
    assert!(r.is_ok());

    check_until(Duration::from_secs(3), || is_ack.load(Ordering::Relaxed))
        .await
        .expect("upload image failed");
}
