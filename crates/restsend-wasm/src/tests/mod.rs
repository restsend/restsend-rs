use wasm_bindgen_test::*;

use crate::js;

#[wasm_bindgen_test]
async fn test_sleep() {
    js::sleep(std::time::Duration::from_millis(100)).await;
}
