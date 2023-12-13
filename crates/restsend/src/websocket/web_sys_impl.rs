use super::{WebSocketCallback, WebsocketOption};
use crate::Result;

pub struct WebSocketImpl {}

impl WebSocketImpl {
    pub fn new() -> Self {
        WebSocketImpl {}
    }

    pub async fn send(&self, message: String) -> Result<()> {
        Ok(())
    }

    pub async fn serve(
        &self,
        opt: &WebsocketOption,
        callback: Box<dyn WebSocketCallback>,
    ) -> Result<()> {
        Ok(())
    }
}
