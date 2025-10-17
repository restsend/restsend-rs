use super::{WebSocketCallback, WebsocketOption};
use crate::error::ClientError;
use crate::utils::elapsed;
use crate::utils::now_millis;
use crate::Result;
use futures_channel::oneshot;
use log::info;
use log::warn;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::{Event, MessageEvent, WebSocket};

struct WebSocketHandlers {
    on_open: Closure<dyn FnMut()>,
    on_message: Closure<dyn FnMut(MessageEvent)>,
    on_error: Closure<dyn FnMut(Event)>,
    on_close: Closure<dyn FnMut(Event)>,
}

pub struct WebSocketImpl {
    ws: RefCell<Option<web_sys::WebSocket>>,
    handlers: RefCell<Option<WebSocketHandlers>>,
}

impl WebSocketImpl {
    pub fn new() -> Self {
        WebSocketImpl {
            ws: RefCell::new(None),
            handlers: RefCell::new(None),
        }
    }

    pub async fn send(&self, message: String) -> Result<()> {
        if let Some(ws) = self.ws.borrow().as_ref() {
            ws.send_with_str(&message)?;
        }
        Ok(())
    }

    pub async fn serve(
        &self,
        opt: &WebsocketOption,
        callback: Box<dyn WebSocketCallback>,
    ) -> Result<()> {
        let mut url = opt.url.replacen("http", "ws", 1);
        let st: i64 = now_millis();

        let current_host = match web_sys::window() {
            Some(window) => window.location().host().unwrap_or_default(),
            None => "".to_string(),
        };

        let is_cross_domain =
            current_host.is_empty() || !url.contains(&current_host) || opt.is_cross_domain;

        if is_cross_domain && !opt.token.is_empty() {
            let mut u = url::Url::parse(&url)
                .map_err(|_| ClientError::HTTP(format!("url parse fail {}", url)))?;
            u.query_pairs_mut().append_pair("token", &opt.token);
            url = u.to_string();
        }

        callback.on_connecting();

        let callback = Rc::new(callback);
        let callback_ref = callback.clone();
        let ws = match WebSocket::new(&url) {
            Ok(ws) => ws,
            Err(e) => {
                let reason = match e.dyn_into::<js_sys::Error>() {
                    Ok(e) => e.message().to_string().as_string(),
                    Err(e) => e.as_string(),
                }
                .unwrap_or("create Websocket fail".to_string());
                warn!("create Websocket fail: {}", reason);
                callback_ref.on_net_broken(reason.clone());
                return Err(ClientError::HTTP(reason));
            }
        };

        if ws.is_undefined() {
            return Err(ClientError::HTTP("create Websocket fail".to_string()));
        }

        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);
        self.ws.borrow_mut().replace(ws.clone());

        let (close_tx, close_rx) = oneshot::channel::<Option<String>>();
        let close_tx = Rc::new(RefCell::new(Some(close_tx)));
        let closed_flag = Rc::new(Cell::new(false));

        let onopen_callback = Closure::<dyn FnMut()>::new({
            let callback_ref = callback.clone();
            move || {
                callback_ref.on_connected(elapsed(st));
            }
        });
        ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));

        let onmessage_callback = Closure::<dyn FnMut(MessageEvent)>::new({
            let callback_ref = callback.clone();
            move |e: MessageEvent| {
                if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                    let array = js_sys::Uint8Array::new(&abuf);
                    let len = array.byte_length() as usize;
                    let mut buf = vec![0u8; len];
                    array.copy_to(&mut buf[..]);
                    if let Ok(message) = String::from_utf8(buf) {
                        callback_ref.on_message(message);
                    }
                } else if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                    if let Some(message) = txt.as_string() {
                        callback_ref.on_message(message);
                    }
                }
            }
        });
        ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));

        let onerror_callback = Closure::<dyn FnMut(Event)>::new({
            let callback_ref = callback.clone();
            let close_tx = Rc::clone(&close_tx);
            let closed_flag = Rc::clone(&closed_flag);
            move |e: Event| {
                let reason = match js_sys::Reflect::get(&e, &JsValue::from_str("reason")) {
                    Ok(v) => v.as_string(),
                    Err(err) => err.as_string(),
                }
                .unwrap_or_default();
                warn!("error event error: {:?}", reason);
                if !closed_flag.replace(true) {
                    callback_ref.on_net_broken(reason.clone());
                    if let Some(tx) = close_tx.borrow_mut().take() {
                        let _ = tx.send(Some(reason));
                    }
                }
            }
        });
        ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));

        let onclose_callback = Closure::<dyn FnMut(Event)>::new({
            let callback_ref = callback.clone();
            let close_tx = Rc::clone(&close_tx);
            let closed_flag = Rc::clone(&closed_flag);
            move |e: Event| {
                let reason = match js_sys::Reflect::get(&e, &JsValue::from_str("reason")) {
                    Ok(v) => v.as_string(),
                    Err(err) => err.as_string(),
                }
                .unwrap_or_default();
                warn!("close event error: {}", reason);
                if !closed_flag.replace(true) {
                    callback_ref.on_net_broken(reason.clone());
                    if let Some(tx) = close_tx.borrow_mut().take() {
                        let _ = tx.send(Some(reason));
                    }
                }
            }
        });
        ws.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));

        self.handlers.borrow_mut().replace(WebSocketHandlers {
            on_open: onopen_callback,
            on_message: onmessage_callback,
            on_error: onerror_callback,
            on_close: onclose_callback,
        });

        let close_reason = close_rx.await.unwrap_or(None);
        warn!(
            "websocket closed: lifetime:{:?}, reason:{:?}",
            elapsed(st),
            close_reason
        );
        self.cleanup();
        Ok(())
    }

    fn cleanup(&self) {
        if let Some(ws) = self.ws.borrow_mut().take() {
            ws.set_onopen(None);
            ws.set_onmessage(None);
            ws.set_onerror(None);
            ws.set_onclose(None);
            let _ = ws.close();
        }
        self.handlers.borrow_mut().take();
    }
}

impl Drop for WebSocketImpl {
    fn drop(&mut self) {
        info!("websocket drop");
        self.cleanup();
    }
}
