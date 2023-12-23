use wasm_bindgen_test::wasm_bindgen_test;

use crate::tests::ENDPOINT;

#[wasm_bindgen_test]
async fn test_create_chat() {
    crate::setLogging(Some("debug".to_string()));
    let info = restsend_sdk::services::auth::login_with_password(
        ENDPOINT.to_string(),
        "bob".to_string(),
        "bob:demo".to_string(),
    )
    .await
    .expect("auth fail");

    let rs_client = crate::Client::new(serde_wasm_bindgen::to_value(&info).unwrap());
    rs_client.connect().await.expect("connect fail");

    let user_id = "alice".to_string();
    let conversation = rs_client.createChat(user_id.clone()).await;
    assert!(!conversation.is_ok());
}
