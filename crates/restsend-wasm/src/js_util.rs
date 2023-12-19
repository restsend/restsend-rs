use crate::CallbackFunction;
use js_sys::JsString;
use restsend_sdk::models::{Attachment, Content};
use std::sync::{Arc, Mutex};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

pub fn get_function(cb: &JsValue, key: &str) -> CallbackFunction {
    let property_key = JsValue::from_str(key);
    let value = js_sys::Reflect::get(&cb, &property_key);
    if let Ok(v) = value {
        if let Ok(v) = v.dyn_into::<js_sys::Function>() {
            return Arc::new(Mutex::new(Some(v)));
        }
    }
    Arc::new(Mutex::new(None))
}

pub fn get_vec_strings(obj: &JsValue, key: &str) -> Option<Vec<String>> {
    let value = js_sys::Reflect::get(&obj, &JsValue::from_str(key));
    if let Ok(v) = value {
        return serde_wasm_bindgen::from_value(v).ok();
    }
    None
}

pub fn get_string(obj: &JsValue, key: &str) -> Option<String> {
    let value = js_sys::Reflect::get(&obj, &JsValue::from_str(key));
    if let Ok(v) = value {
        if let Ok(v) = v.dyn_into::<JsString>() {
            return Some(v.as_string().unwrap_or_default());
        }
    }
    None
}

#[allow(unused)]
pub fn get_f64(obj: &JsValue, key: &str) -> f64 {
    let value = js_sys::Reflect::get(&obj, &JsValue::from_str(key));
    if let Ok(v) = value {
        if let Ok(v) = v.dyn_into::<js_sys::Number>() {
            return v.as_f64().unwrap_or_default();
        }
    }
    0.0
}

pub fn get_bool(obj: &JsValue, key: &str) -> bool {
    let value = js_sys::Reflect::get(&obj, &JsValue::from_str(key));
    if let Ok(v) = value {
        if let Ok(v) = v.dyn_into::<js_sys::Boolean>() {
            return v.as_bool().unwrap_or_default();
        }
    }
    false
}

pub fn js_value_to_attachment(obj: &JsValue) -> Option<Attachment> {
    let url = get_string(obj, "url");
    let is_private = get_bool(obj, "private");
    let size = get_f64(obj, "size") as i64;

    match url {
        Some(v) => return Some(Attachment::from_url(&v, is_private, size)),
        None => {}
    }
    match js_sys::Reflect::get(&obj, &JsValue::from_str("file")) {
        Ok(v) => {
            if v.is_instance_of::<web_sys::File>() {
                let f = v.dyn_into::<web_sys::File>().unwrap();
                let file_name = f.name();
                let file_size = f.size() as i64;

                return Some(Attachment::from_blob(
                    f.into(),
                    Some(file_name),
                    is_private,
                    file_size,
                ));
            } else {
                match v.dyn_into::<web_sys::Blob>() {
                    Ok(b) => {
                        let file_size = b.size() as i64;
                        return Some(Attachment::from_blob(b, None, is_private, file_size));
                    }
                    Err(_) => {}
                }
            }
        }
        Err(_) => {}
    }
    None
}

pub fn js_value_to_content(obj: JsValue) -> Option<Content> {
    let attachment = match js_sys::Reflect::get(&obj, &JsValue::from_str("attachment")) {
        Ok(v) => js_value_to_attachment(&v),
        Err(_) => None,
    };

    let mut content = match serde_wasm_bindgen::from_value::<Content>(obj).ok() {
        Some(v) => v,
        None => return None,
    };

    content.attachment = attachment;
    Some(content)
}
