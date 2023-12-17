use super::{WebSocketCallback, WebsocketOption};
use crate::error::ClientError;
use crate::utils::now_millis;
use crate::Result;
use log::{debug, warn};
use std::sync::Arc;
use std::sync::Mutex;
use tokio::select;
use tokio::sync::mpsc::unbounded_channel;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;
use wasm_bindgen::closure::*;
use wasm_bindgen::JsCast;
use web_sys::{ErrorEvent, MessageEvent, WebSocket};

pub struct WebSocketImpl {
    sender_tx: Mutex<UnboundedSender<String>>,
    sender_rx: Mutex<Option<UnboundedReceiver<String>>>,
}

enum WSEvent {
    Closed(String),
    Opened,
    Message(String),
}

impl WebSocketImpl {
    pub fn new() -> Self {
        let (sender_tx, sender_rx) = unbounded_channel::<String>();
        WebSocketImpl {
            sender_tx: Mutex::new(sender_tx),
            sender_rx: Mutex::new(Some(sender_rx)),
        }
    }

    pub async fn send(&self, message: String) -> Result<()> {
        self.sender_tx.lock().unwrap().send(message)?;
        Ok(())
    }

    pub async fn serve(
        &self,
        opt: &WebsocketOption,
        callback: Box<dyn WebSocketCallback>,
    ) -> Result<()> {
        let mut url = opt.url.replace("http", "ws");
        let st = now_millis();
        callback.on_connecting();

        let current_host = match web_sys::window() {
            Some(window) => window.location().host().unwrap_or_default(),
            None => "".to_string(),
        };

        let is_cross_domain = current_host.is_empty() || !url.contains(&current_host);

        if is_cross_domain && !opt.token.is_empty() {
            let mut u = url::Url::parse(&url).unwrap();
            u.query_pairs_mut().append_pair("token", &opt.token);
            url = u.to_string();
        }

        let ws = match WebSocket::new(&url) {
            Ok(ws) => ws,
            Err(e) => {
                let reason = e.as_string().unwrap_or("WebSocket create fail".to_string());
                callback.on_net_broken(reason.clone());
                return Err(ClientError::HTTP(reason));
            }
        };

        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

        let (tx, mut rx) = unbounded_channel::<WSEvent>();
        let tx = Arc::new(tx);
        let tx_ref = tx.clone();

        let onerror_callback = Closure::<dyn FnMut(_)>::new(move |e: ErrorEvent| {
            let reason = e.error().as_string().unwrap_or("unknown".to_string());
            warn!("error event error: {:?}", reason);
            tx_ref.send(WSEvent::Closed(reason)).ok();
        });
        ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
        onerror_callback.forget();

        let cloned_ws = ws.clone();
        let tx_ref: Arc<UnboundedSender<WSEvent>> = tx.clone();
        let onopen_callback = Closure::<dyn FnMut()>::new(move || {
            tx_ref.send(WSEvent::Opened).ok();
        });
        cloned_ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
        onopen_callback.forget();

        let tx_ref: Arc<UnboundedSender<WSEvent>> = tx.clone();
        let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
            if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                let array = js_sys::Uint8Array::new(&abuf);
                let len = array.byte_length() as usize;
                let mut buf = vec![0u8; len];
                array.copy_to(&mut buf[..]);
                let message = String::from_utf8(buf);
                match message {
                    Ok(message) => {
                        tx_ref.send(WSEvent::Message(message)).ok();
                    }
                    Err(e) => {
                        debug!("message event, received arraybuffer: {:?}", e);
                    }
                }
            } else if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                let message = txt.as_string();
                match message {
                    Some(message) => {
                        tx_ref.send(WSEvent::Message(message)).ok();
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
        let rx_loop = async move {
            loop {
                let event = rx.recv().await;
                match event {
                    Some(WSEvent::Closed(reason)) => {
                        debug!("websocket closed {}", reason);
                        return Err(ClientError::HTTP(reason));
                    }
                    Some(WSEvent::Opened) => {
                        debug!("websocket opened");
                        let usage = crate::utils::elapsed(st);
                        callback.on_connected(usage);
                    }
                    Some(WSEvent::Message(msg)) => {
                        callback.on_message(msg);
                    }
                    None => {
                        debug!("websocket recv None");
                        return Err(ClientError::HTTP(format!("websocket recv None")));
                    }
                }
            }
        };

        let ws_ref = ws.clone();
        let sender_rx = self.sender_rx.lock().unwrap().take();
        let send_loop = async move {
            let mut sender_rx = sender_rx.unwrap();
            loop {
                let msg = match sender_rx.recv().await {
                    Some(msg) => msg,
                    None => {
                        debug!("websocket send close");
                        return Ok(());
                    }
                };
                debug!("websocket send: {}", msg);
                let r = ws_ref.send_with_str(&msg);
                match r {
                    Ok(_) => {}
                    Err(e) => {
                        let reason = e.as_string().unwrap_or("unknown".to_string());
                        return Err(ClientError::HTTP(format!(
                            "websocket send failed: {}",
                            reason
                        )));
                    }
                }
            }
        };

        let r = select! {
            r = rx_loop => {
                warn!("websocket rx_loop exit");
                r
            },
            r = send_loop =>{
                warn!("websocket send_loop exit");
                r
            },
        };
        r
    }
}
