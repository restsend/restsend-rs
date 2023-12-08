use super::RsClient;
use crate::callback_wrap::*;
use std::sync::Arc;

#[uniffi::export]
impl RsClient {
    pub async fn download_file(
        self: Arc<Self>,
        url: String,
        callback: Box<dyn RSDownloadCallback>,
    ) -> Option<String> {
        self.0
            .download_file(&url, Box::new(RSDownloadCallbackWrap { 0: callback }))
            .await
            .ok()
    }
}
