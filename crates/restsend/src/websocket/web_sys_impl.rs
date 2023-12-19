use super::{WebSocketCallback, WebsocketOption};
use crate::error::ClientError;
use crate::utils::elapsed;
use crate::utils::now_millis;
use crate::Result;
use log::{debug, warn};
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::oneshot;
use wasm_bindgen::closure::*;
use wasm_bindgen::JsCast;
use web_sys::{ErrorEvent, MessageEvent, WebSocket};

pub struct WebSocketImpl {
    ws: Mutex<Option<web_sys::WebSocket>>,
}

impl WebSocketImpl {
    pub fn new() -> Self {
        WebSocketImpl {
            ws: Mutex::new(None),
        }
    }

    pub async fn send(&self, message: String) -> Result<()> {
        log::warn!("websocket send: {:?}", message);

        if let Some(ws) = self.ws.lock().unwrap().as_ref() {
            match ws.send_with_str(&message) {
                Ok(_) => {}
                Err(e) => {
                    // get error message from JsValue
                    let reason = match e.dyn_into::<js_sys::Error>() {
                        Ok(e) => e.message().as_string(),
                        Err(e) => e.as_string(),
                    };
                    warn!(
                        "websocket send error: {:?} discard message:{:?}",
                        reason, message
                    );
                }
            }
        } else {
            warn!("websocket is not connected, discard message: {:?}", message);
        }
        Ok(())
    }

    pub async fn serve(
        &self,
        opt: &WebsocketOption,
        callback: Box<dyn WebSocketCallback>,
    ) -> Result<()> {
        let mut url = opt.url.replace("http", "ws");
        let st: i64 = now_millis();

        let current_host = match web_sys::window() {
            Some(window) => window.location().host().unwrap_or_default(),
            None => "".to_string(),
        };

        let is_cross_domain = current_host.is_empty() || !url.contains(&current_host);

        if is_cross_domain && !opt.token.is_empty() {
            let mut u = url::Url::parse(&url)
                .map_err(|_| ClientError::HTTP(format!("url parse fail {}", url)))?;
            u.query_pairs_mut().append_pair("token", &opt.token);
            url = u.to_string();
        }

        callback.on_connecting();

        let callback = Arc::new(Mutex::new(callback));
        let callback_ref = callback.clone();
        let ws = match WebSocket::new(&url) {
            Ok(ws) => ws,
            Err(e) => {
                let reason = match e.dyn_into::<js_sys::Error>() {
                    Ok(e) => e.message().to_string().as_string(),
                    Err(e) => e.as_string(),
                }
                .unwrap_or("create Websocket fail".to_string());

                callback_ref
                    .lock()
                    .unwrap()
                    .as_ref()
                    .on_net_broken(reason.clone());
                return Err(ClientError::HTTP(reason));
            }
        };
        log::warn!("websocket url: {}", url);
        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

        let callback_ref = callback.clone();
        let onopen_callback = Closure::<dyn FnMut()>::new(move || {
            callback_ref
                .lock()
                .unwrap()
                .as_ref()
                .on_connected(elapsed(st));
        });
        ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
        onopen_callback.forget();

        let callback_ref = callback.clone();
        let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
            if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                let array = js_sys::Uint8Array::new(&abuf);
                let len = array.byte_length() as usize;
                let mut buf = vec![0u8; len];
                array.copy_to(&mut buf[..]);
                let message = String::from_utf8(buf);
                match message {
                    Ok(message) => {
                        callback_ref.lock().unwrap().as_ref().on_message(message);
                    }
                    Err(e) => {
                        debug!("message event, received arraybuffer: {:?}", e);
                    }
                }
            } else if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                let message = txt.as_string();
                match message {
                    Some(message) => {
                        callback_ref.lock().unwrap().as_ref().on_message(message);
                    }
                    None => {
                        debug!("message event, received Text: {:?}", txt);
                    }
                }
            } else {
                debug!("message event, received Unknown: {:?}", e.data());
            }
        });
        ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        onmessage_callback.forget();

        let (close_tx, close_rx) = oneshot::channel::<()>();
        let callback_ref = callback.clone();
        let close_tx = Mutex::new(Some(close_tx));
        let onerror_callback = Closure::<dyn FnMut(_)>::new(move |e: ErrorEvent| {
            let reason = e.message();
            warn!("error event error: {:?}", reason);
            callback_ref.lock().unwrap().as_ref().on_net_broken(reason);
            if let Some(close_tx) = close_tx.lock().unwrap().take() {
                close_tx.send(()).ok();
            }
        });
        ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
        onerror_callback.forget();
        self.ws.lock().unwrap().replace(ws);
        close_rx.await.ok();
        log::warn!("websocket closed: lifetime:{:?}", elapsed(st));
        Ok(())
    }
}
