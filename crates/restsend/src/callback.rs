pub trait Callback: Send + Sync {
    fn on_connected(&self) {}
    fn on_connecting(&self) {}
    fn on_token_expired(&self, reason: String) {}
    fn on_net_broken(&self, reason: String) {}
    fn on_kickoff_by_other_client(&self, reason: String) {}
}
