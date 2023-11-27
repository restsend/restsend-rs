use crate::error::ClientError::{HTTPError, TokenExpired};
use anyhow::Result;
use futures_util::StreamExt;
use log::{debug, warn};
use reqwest::header::{ACCEPT, AUTHORIZATION, USER_AGENT};

use tokio::select;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::time::{sleep, Duration, Instant};
use tokio_websockets::ClientBuilder;

pub(super) struct WebSocketInner {
    sender_tx: UnboundedSender<String>,
    sender_rx: UnboundedReceiver<String>,
}

impl super::WebSocket {
    pub fn new() -> Self {
        let (sender_tx, sender_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        Self {
            inner: WebSocketInner {
                sender_tx,
                sender_rx,
            },
        }
    }

    pub async fn send(&mut self, message: String) -> Result<()> {
        self.inner.sender_tx.send(message)?;
        Ok(())
    }

    pub async fn serve(
        &mut self,
        opt: &super::WebsocketOption,
        callback: Box<dyn super::WebSocketCallback>,
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

        let (mut stream, resp) = select! {
            r = req.connect() => {
                r
            },
            _ = sleep(Duration::from_secs(10)) => {
                warn!("websocket connect timeout");
                let reason = format!("websocket connect timeout");
                callback.on_net_broken(reason.clone());
                return Err(HTTPError(reason).into());
            },
        }?;

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

        let recv_loop = async {
            loop {
                let msg = match stream.next().await {
                    Some(Ok(msg)) => Ok(msg),
                    Some(Err(e)) => Err(HTTPError(format!("websocket recv failed: {}", e))),
                    None => Err(HTTPError(format!("websocket recv None"))),
                }?;

                if msg.is_ping() {
                    debug!("websocket recv ping");
                    continue;
                }
                if msg.is_pong() {
                    debug!("websocket recv pong");
                    continue;
                }

                if msg.is_close() {
                    break;
                }

                let body = {
                    if msg.is_binary() {
                        let data: Vec<u8> = msg.as_payload().to_vec();
                        String::from_utf8(data).unwrap()
                    } else {
                        msg.as_text().unwrap().to_string()
                    }
                };
                callback.on_message(body);
            }
        };

        Ok(())
    }
}
