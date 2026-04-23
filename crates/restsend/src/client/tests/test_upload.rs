use crate::{
    callback,
    client::{
        tests::{test_client::TestMessageCakllbackImpl, test_endpoint, unique_test_user},
        Client,
    },
    models::Attachment,
    request::ChatRequest,
    services::auth::{login_with_password, signup},
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
    let user_a = unique_test_user("sdk-upload-a");
    let user_b = unique_test_user("sdk-upload-b");
    let password = "pass-1".to_string();
    signup(test_endpoint(), user_a.clone(), password.clone())
        .await
        .expect("signup upload sender");
    signup(test_endpoint(), user_b.clone(), password.clone())
        .await
        .expect("signup upload receiver");

    let info = login_with_password(
        test_endpoint(),
        user_a.clone(),
        password,
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

    let topic_id = c
        .create_chat(user_b)
        .await
        .expect("create chat before upload")
        .topic_id;

    let r = c
        .do_send_image(
            topic_id,
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
