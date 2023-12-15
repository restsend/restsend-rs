use js_sys::JsString;
use restsend_sdk::models::Attachment;
use restsend_sdk::models::Content;
use restsend_sdk::models::ContentType;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

pub fn get_vec_strings(obj: &JsValue, key: &str) -> Option<Vec<String>> {
    let value = js_sys::Reflect::get(&obj, &JsValue::from_str(key));
    if let Ok(v) = value {
        if let Ok(v) = v.dyn_into::<js_sys::Array>() {
            let mut mentions = Vec::new();
            for i in 0..v.length() {
                if let Ok(v) = v.get(i).dyn_into::<JsString>() {
                    mentions.push(v.as_string().unwrap_or_default());
                }
            }
            return Some(mentions);
        }
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
    let is_private = get_bool(obj, "isPrivate");
    match url {
        Some(v) => return Some(Attachment::from_url(&v, is_private)),
        None => {}
    }
    match js_sys::Reflect::get(&obj, &JsValue::from_str("file")) {
        Ok(v) => {
            if v.is_instance_of::<web_sys::File>() {
                let f = v.dyn_into::<web_sys::File>().unwrap();
                let file_name = f.name();
                return Some(Attachment::from_blob(f.into(), Some(file_name), is_private));
            } else {
                match v.dyn_into::<web_sys::Blob>() {
                    Ok(b) => {
                        return Some(Attachment::from_blob(b, None, is_private));
                    }
                    Err(_) => {}
                }
            }
        }
        Err(_) => {}
    }
    None
}

pub fn js_value_to_content(obj: &JsValue) -> Option<Content> {
    let content_type = match get_string(obj, "type") {
        Some(v) => match v.as_str() {
            "text" => Some(ContentType::Text),
            "image" => Some(ContentType::Image),
            "video" => Some(ContentType::Video),
            "file" => Some(ContentType::File),
            _ => None,
        },
        None => None,
    };

    if content_type.is_none() {
        return None;
    }

    let text = get_string(obj, "text");
    let placeholder = get_string(obj, "placeholder");
    let thumbnail = get_string(obj, "thumbnail");
    let duration = get_string(obj, "duration");
    let size = get_f64(obj, "size");
    let width = get_f64(obj, "width");
    let height = get_f64(obj, "height");
    let mentions = get_vec_strings(obj, "mentions");
    let attachment = js_sys::Reflect::get(&obj, &JsValue::from_str("attachment"))
        .map(|v| js_value_to_attachment(&v).unwrap_or_default())
        .ok();

    let mut content = Content::new_text(content_type.unwrap(), &text.unwrap_or_default());
    content.placeholder = placeholder.unwrap_or_default();
    content.thumbnail = thumbnail.unwrap_or_default();
    content.duration = duration.unwrap_or_default();
    content.size = size as u64;
    content.width = width as f32;
    content.height = height as f32;
    content.mentions = mentions.unwrap_or_default();
    content.attachment = attachment;

    Some(content)
}
