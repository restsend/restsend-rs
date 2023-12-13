use wasm_bindgen_test::*;

use crate::js;

#[wasm_bindgen_test]
async fn test_sleep() {
    js::sleep(std::time::Duration::from_millis(100)).await;
}

#[wasm_bindgen_test]
async fn test_new_client() {
    let rs_client = crate::RsClient::new(
        "https://chat.ruzhila.cn".to_string(),
        "bob".to_string(),
        "bad_token".to_string(),
    );
    assert_eq!(rs_client.0.endpoint, "https://chat.ruzhila.cn");
}
