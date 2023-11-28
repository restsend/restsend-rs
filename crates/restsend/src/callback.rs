use anyhow::Error;

#[allow(unused_variables)]
pub trait Callback: Send + Sync {
    fn on_connected(&self) {}
    fn on_connecting(&self) {}
    fn on_token_expired(&self, reason: String) {}
    fn on_net_broken(&self, reason: String) {}
    fn on_kickoff_by_other_client(&self, reason: String) {}
}

#[allow(unused_variables)]
pub trait UploadCallback: Send + Sync {
    fn on_progress(&self, progress: u64, total: u64) {}
    fn on_success(&self, url: String) {}
    fn on_fail(&self, e: Error) {}
}

#[allow(unused_variables)]
pub trait DownloadCallback: Send + Sync {
    fn on_progress(&self, progress: u64, total: u64) {}
    fn on_success(&self, url: String, file_name: String) {}
    fn on_fail(&self, e: Error) {}
}
