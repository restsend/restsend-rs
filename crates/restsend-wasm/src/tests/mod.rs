use restsend_sdk::utils::sleep;
use wasm_bindgen_test::*;
wasm_bindgen_test_configure!(run_in_browser);

const ENDPOINT: &str = "https://chat.ruzhila.cn";
#[wasm_bindgen_test]
async fn test_sleep() {
    sleep(std::time::Duration::from_millis(100)).await;
}

#[wasm_bindgen_test]
async fn test_new_client() {
    let rs_client = crate::Client::new(
        ENDPOINT.to_string(),
        "bob".to_string(),
        "bad_token".to_string(),
    );
    assert_eq!(rs_client.inner.endpoint, ENDPOINT);
}

#[wasm_bindgen_test]
async fn test_auth() {
    crate::account::signin(
        ENDPOINT.to_string(),
        "bob".to_string(),
        Some("bob:demo".to_string()),
        None,
    )
    .await
    .expect("auth fail");
}

#[wasm_bindgen_test]
async fn test_connect() {
    crate::enable_logging(Some("debug".to_string()));
    let info = restsend_sdk::services::auth::login_with_password(
        ENDPOINT.to_string(),
        "bob".to_string(),
        "bob:demo".to_string(),
    )
    .await
    .expect("auth fail");

    let rs_client = crate::Client::new(ENDPOINT.to_string(), "bob".to_string(), info.token);
    rs_client.connect().await.expect("connect fail");
}
