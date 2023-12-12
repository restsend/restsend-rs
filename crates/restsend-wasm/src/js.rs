use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    pub fn console_log(s: &str);

    #[wasm_bindgen(js_name = setTimeout)]
    pub fn set_timeout(closure: &wasm_bindgen::closure::Closure<dyn FnMut()>, time: u32);
    #[wasm_bindgen(js_name = clearTimeout)]
    pub fn clear_timeout(id: i32);
}

pub async fn sleep(d: std::time::Duration) {
    let p = js_sys::Promise::new(&mut |resolve, _| {
        let closure = Closure::new(move || {
            let this = JsValue::null();
            let _ = resolve.call0(&this);
        });
        set_timeout(&closure, d.as_millis() as u32);
        closure.forget();
    });
    wasm_bindgen_futures::JsFuture::from(p).await.ok();
}
