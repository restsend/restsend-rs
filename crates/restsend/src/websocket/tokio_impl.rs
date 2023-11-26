use crate::error::ClientError;
use anyhow::Result;
use http::request::Builder;
use log::warn;
use tokio::select;
use tokio::time::{sleep, Duration, Instant};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

impl super::WebSocket {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn connect(
        &mut self,
        opt: &super::WebsocketOption,
        callback: Box<dyn super::WebSocketCallback>,
    ) -> Result<()> {
        let st = Instant::now();
        let req = Builder::new()
            .header("Authorization", format!("Bearer {}", opt.token))
            .header("User-Agent", crate::USER_AGENT)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .method("GET")
            .uri(&opt.url)
            .body(())
            .unwrap();

        let (ws_stream, _) = connect_async(req).await?;
        /*
        let r = select! {
            _ = async {
                sleep(opt.handshake_timeout).await
            } => {
                warn!(
                    "websocket connect timeout elapsed:{} ms",
                    st.elapsed().as_millis()
                );
                Err(ClientError::WebsocketError("connect timeout".to_string()))
            }
            r = connect_async(req) => {
                Ok(r)
            }
        }?;
             */
        Ok(())
    }
}
