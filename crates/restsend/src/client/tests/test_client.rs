use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use super::check_until;
use crate::{
    callback,
    client::{tests::TEST_ENDPOINT, Client},
    services::auth::login_with_password,
};
#[tokio::test]
async fn test_client_init() {
    let info = login_with_password(TEST_ENDPOINT, "bob", "bob:demo").await;
    assert!(info.is_ok());

    let c = Client::new("", "", &info.unwrap());
    _ = c;

    let is_connected = Arc::new(AtomicBool::new(false));

    struct CallbackImpl {
        is_connected: Arc<AtomicBool>,
    }

    impl callback::Callback for CallbackImpl {
        fn on_connected(&self) {
            println!("on_connected,");
            self.is_connected.store(true, Ordering::Relaxed);
        }
        fn on_connecting(&self) {
            println!("on_connecting");
        }
        fn on_net_broken(&self, reason: String) {
            println!("on_net_broken: {}", reason);
        }
    }

    let callback = Box::new(CallbackImpl {
        is_connected: is_connected.clone(),
    });

    c.connect(callback).await;

    check_until(Duration::from_secs(3), || {
        is_connected.load(Ordering::Relaxed)
    })
    .await
    .unwrap();
}
