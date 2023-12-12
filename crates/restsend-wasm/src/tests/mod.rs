use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
async fn test_rsclient() {
    let client = crate::RsClient::new();
    client.connect(JsValue::NULL).await;
}
