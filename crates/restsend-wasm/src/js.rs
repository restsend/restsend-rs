use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    pub fn console_log(s: &str);

    #[wasm_bindgen(js_name = setTimeout)]
    pub fn set_timeout(closure: &wasm_bindgen::closure::Closure<dyn FnMut()>, time: u32) -> i32;
    #[wasm_bindgen(js_name = clearTimeout)]
    pub fn clear_timeout(id: i32);
}
