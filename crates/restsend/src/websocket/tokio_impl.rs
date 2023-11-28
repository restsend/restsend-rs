use crate::error::ClientError::{HTTPError, TokenExpired};
use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use log::{debug, warn};
use reqwest::header::{ACCEPT, AUTHORIZATION, USER_AGENT};

use tokio::select;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::time::{sleep, Instant};
use tokio_websockets::{ClientBuilder, Message};

use super::{WebSocketCallback, WebsocketOption};

pub(super) struct WebSocketInner {
    sender_tx: UnboundedSender<String>,
    sender_rx: Option<UnboundedReceiver<String>>,
}

pub struct WebSocketImpl {
    inner: WebSocketInner,
}

impl WebSocketImpl {
    pub fn new() -> Self {
        let (sender_tx, sender_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        Self {
            inner: WebSocketInner {
                sender_tx,
                sender_rx: Some(sender_rx),
            },
        }
    }

    pub async fn send(&mut self, message: String) -> Result<()> {
        self.inner.sender_tx.send(message)?;
        Ok(())
    }

    pub async fn serve(
        &mut self,
        opt: &WebsocketOption,
        callback: Box<dyn WebSocketCallback>,
    ) -> Result<()> {
        let url = opt.url.replace("http", "ws");

        let req = ClientBuilder::new()
            .add_header(
                AUTHORIZATION,
                format!("Bearer {}", opt.token).parse().unwrap(),
            )
            .add_header(USER_AGENT, crate::USER_AGENT.parse().unwrap())
            .add_header(ACCEPT, "application/json".parse().unwrap())
            .uri(&url)
            .unwrap();

        let st = Instant::now();
        callback.on_connecting();

        let resp = select! {
            r = req.connect() => {
                r
            },
            _ = sleep(opt.handshake_timeout) => {
                return Err(HTTPError(format!("websocket connect timeout")).into());
            },
        };

        let (stream, resp) = match resp {
            Ok(v) => v,
            Err(e) => {
                warn!("websocket connect failed: {}", e);
                let reason = format!("websocket connect failed: {}", e);
                callback.on_net_broken(reason.clone());
                return Err(HTTPError(reason).into());
            }
        };

        let usage = st.elapsed();
        match resp.status() {
            reqwest::StatusCode::OK => {
                debug!("websocket connected usage: {:?}", st.elapsed());
                callback.on_connected(usage);
            }
            reqwest::StatusCode::UNAUTHORIZED => {
                let reason = format!("websocket unauthorized failed: {}", resp.status());
                warn!("websocket unauthorized failed: {}", resp.status());
                callback.on_unauthorized();
                return Err(TokenExpired(reason).into());
            }
            _ => {
                warn!("websocket connect failed: {}", resp.status());
                let reason = format!("websocket connect failed: {}", resp.status());
                callback.on_net_broken(reason.clone());
                return Err(HTTPError(reason).into());
            }
        }

        let (mut stream_tx, mut stream_rx) = stream.split();
        let recv_loop = async {
            loop {
                let msg = match stream_rx.next().await {
                    Some(Ok(msg)) => msg,
                    Some(Err(e)) => {
                        return Err(HTTPError(format!("websocket recv failed: {}", e)));
                    }
                    None => {
                        return Err(HTTPError(format!("websocket recv None")));
                    }
                };

                if msg.is_ping() {
                    debug!("websocket recv ping");
                    continue;
                }

                if msg.is_pong() {
                    debug!("websocket recv pong");
                    continue;
                }

                if msg.is_close() {
                    debug!("websocket recv close");
                    return Ok(());
                }

                let body = {
                    if msg.is_binary() {
                        String::from_utf8(msg.as_payload().to_vec()).unwrap()
                    } else {
                        msg.as_text().unwrap().to_string()
                    }
                };
                debug!("websocket recv: {}", body);
                callback.on_message(body);
            }
        };

        let sender_rx = self.inner.sender_rx.take();
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
                let r = stream_tx.send(Message::text(msg)).await;
                if let Err(e) = r {
                    return Err(HTTPError(format!("websocket send failed: {}", e)));
                }
            }
        };

        let r = select! {
            r = recv_loop => {
                r
            },
            r = send_loop => {
                r
            },
        };

        let reason = match r {
            Ok(_) => "websocket closed".to_string(),
            Err(e) => e.to_string(),
        };
        warn!("websocket closed: {} lifetime:{:?}", reason, st.elapsed());
        callback.on_net_broken(reason);
        Ok(())
    }
}
