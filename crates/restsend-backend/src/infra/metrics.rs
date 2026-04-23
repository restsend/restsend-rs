use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct RuntimeMetrics {
    inbound_ws_messages: Arc<AtomicU64>,
    outbound_ws_messages: Arc<AtomicU64>,
    webhook_deliveries: Arc<AtomicU64>,
    webhook_failures: Arc<AtomicU64>,
}

impl RuntimeMetrics {
    pub fn incr_inbound_ws_messages(&self) {
        self.inbound_ws_messages.fetch_add(1, Ordering::Relaxed);
    }

    pub fn incr_outbound_ws_messages(&self) {
        self.outbound_ws_messages.fetch_add(1, Ordering::Relaxed);
    }

    pub fn incr_webhook_deliveries(&self) {
        self.webhook_deliveries.fetch_add(1, Ordering::Relaxed);
    }

    pub fn incr_webhook_failures(&self) {
        self.webhook_failures.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> RuntimeMetricsSnapshot {
        RuntimeMetricsSnapshot {
            inbound_ws_messages: self.inbound_ws_messages.load(Ordering::Relaxed),
            outbound_ws_messages: self.outbound_ws_messages.load(Ordering::Relaxed),
            webhook_deliveries: self.webhook_deliveries.load(Ordering::Relaxed),
            webhook_failures: self.webhook_failures.load(Ordering::Relaxed),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeMetricsSnapshot {
    pub inbound_ws_messages: u64,
    pub outbound_ws_messages: u64,
    pub webhook_deliveries: u64,
    pub webhook_failures: u64,
}
