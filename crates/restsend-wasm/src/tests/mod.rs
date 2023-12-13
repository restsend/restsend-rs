use wasm_bindgen_test::*;

use crate::js;
const ENDPOINT: &str = "https://chat.ruzhila.cn";
#[wasm_bindgen_test]
async fn test_sleep() {
    js::sleep(std::time::Duration::from_millis(100)).await;
}

#[wasm_bindgen_test]
async fn test_new_client() {
    let rs_client = crate::Client::new(
        ENDPOINT.to_string(),
        "bob".to_string(),
        "bad_token".to_string(),
    );
    assert_eq!(rs_client.0.endpoint, ENDPOINT);
}

#[wasm_bindgen_test]
async fn test_auth() {
    crate::login_with_password(
        ENDPOINT.to_string(),
        "bob".to_string(),
        "bob:demo".to_string(),
    )
    .await
    .expect("auth fail");
}

#[wasm_bindgen_test]
async fn test_connect() {
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
