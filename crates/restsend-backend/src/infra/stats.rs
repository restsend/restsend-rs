use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use chrono::{Datelike, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    QueryOrder, QuerySelect,
};

use crate::entity::stats_daily;

/// Known metric names (Go stats parity subset).
pub const METRIC_CHAT: &str = "chat";
pub const METRIC_MSG_IN: &str = "msg.received";
pub const METRIC_WS_IN: &str = "ws.messages.in";
pub const METRIC_WS_OUT: &str = "ws.messages.out";
pub const METRIC_WEBHOOK_OK: &str = "webhook.delivered";
pub const METRIC_WEBHOOK_FAIL: &str = "webhook.failed";
pub const METRIC_TOPIC_CREATE: &str = "topic.created";
pub const METRIC_TOPIC_DISMISS: &str = "topic.dismissed";
pub const METRIC_MEMBER_ADD: &str = "topic.member.added";
pub const METRIC_MEMBER_REMOVE: &str = "topic.member.removed";
pub const METRIC_UPLOAD: &str = "upload.count";
pub const METRIC_UPLOAD_BYTES: &str = "upload.bytes";
pub const METRIC_GUEST: &str = "guest.created";
pub const METRIC_USER_BLOCKED: &str = "user.blocked";
pub const METRIC_ONLINE_SECONDS: &str = "online.duration.seconds";
pub const METRIC_KICKOFF: &str = "user.kickoff";

#[derive(Default)]
struct DailyCounters {
    counters: Mutex<HashMap<(String, String), i64>>,
}

/// Daily aggregated counters with periodic flush to the `stats_daily` table.
///
/// Recording is lock-cheap and never blocks the caller on the database; the
/// flush task upserts increments every `flush_interval`.
#[derive(Clone)]
pub struct StatsService {
    db: DatabaseConnection,
    daily: Arc<DailyCounters>,
    pub realtime: Arc<RealtimeGauges>,
    stats_enabled: bool,
}

#[derive(Default)]
pub struct RealtimeGauges {
    pub online_sessions: AtomicU64,
    pub online_users: AtomicU64,
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyStat {
    pub date: String,
    pub metric: String,
    pub value: i64,
}

impl StatsService {
    pub fn new(db: DatabaseConnection, stats_enabled: bool) -> Self {
        Self {
            db,
            daily: Arc::new(DailyCounters::default()),
            realtime: Arc::new(RealtimeGauges::default()),
            stats_enabled,
        }
    }

    pub fn record(&self, metric: &str, value: i64) {
        if !self.stats_enabled {
            return;
        }
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let mut counters = self.daily.counters.lock().unwrap();
        *counters
            .entry((today, metric.to_string()))
            .or_insert(0) += value;
    }

    pub fn record_event(&self, metric: &str) {
        self.record(metric, 1);
    }

    /// Flush in-memory counters into the stats_daily table (upsert add).
    pub async fn flush(&self) -> usize {
        if !self.stats_enabled {
            return 0;
        }
        let snapshot: Vec<(String, String, i64)> = {
            let counters = self.daily.counters.lock().unwrap();
            counters
                .iter()
                .map(|((date, metric), value)| (date.clone(), metric.clone(), *value))
                .collect()
        };
        // Reset what we snapshot so values move to the DB exactly once per
        // flush cycle (upsert adds them onto persisted totals).
        {
            let mut counters = self.daily.counters.lock().unwrap();
            for (date, metric) in snapshot.iter().map(|(d, m, _)| (d.clone(), m.clone())) {
                counters.remove(&(date, metric));
            }
        }

        let mut flushed = 0;
        for (date, metric, delta) in snapshot {
            if delta == 0 {
                continue;
            }
            let existing = stats_daily::Entity::find_by_id((date.clone(), metric.clone()))
                .one(&self.db)
                .await
                .ok()
                .flatten();
            match existing {
                Some(row) => {
                    let new_value = row.value + delta;
                    let mut active = row.into_active_model();
                    active.value = sea_orm::ActiveValue::Set(new_value);
                    if active.update(&self.db).await.is_ok() {
                        flushed += 1;
                    }
                }
                None => {
                    let active = stats_daily::ActiveModel {
                        date: sea_orm::ActiveValue::Set(date),
                        metric: sea_orm::ActiveValue::Set(metric),
                        value: sea_orm::ActiveValue::Set(delta),
                    };
                    if active.insert(&self.db).await.is_ok() {
                        flushed += 1;
                    }
                }
            }
        }
        flushed
    }

    /// Start the periodic flush loop (every 60s).
    pub fn start_flush_loop(&self) {
        let service = self.clone();
        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                let flushed = service.flush().await;
                if flushed > 0 {
                    tracing::debug!(flushed, "stats flush");
                }
            }
        });
    }

    /// Persisted daily stats for the last `days` days.
    pub async fn recent_daily(&self, days: u32) -> Vec<DailyStat> {
        let since = (Utc::now() - chrono::Duration::days(days as i64))
            .format("%Y-%m-%d")
            .to_string();
        let rows: Vec<stats_daily::Model> = stats_daily::Entity::find()
            .filter(stats_daily::Column::Date.gte(since))
            .order_by_asc(stats_daily::Column::Date)
            .all(&self.db)
            .await
            .unwrap_or_default();
        rows.into_iter()
            .map(|row| DailyStat {
                date: row.date,
                metric: row.metric,
                value: row.value,
            })
            .collect()
    }

    /// Prometheus text exposition of realtime gauges + counters.
    pub fn render_prometheus(&self) -> String {
        let snapshot = (
            self.realtime.online_sessions.load(Ordering::Relaxed),
            self.realtime.online_users.load(Ordering::Relaxed),
        );
        let daily: Vec<(String, i64)> = {
            let counters = self.daily.counters.lock().unwrap();
            let mut totals: HashMap<String, i64> = HashMap::new();
            for ((_date, metric), value) in counters.iter() {
                *totals.entry(metric.clone()).or_insert(0) += value;
            }
            let mut out: Vec<(String, i64)> = totals.into_iter().collect();
            out.sort();
            out
        };

        let mut out = String::new();
        out.push_str("# HELP restsend_online_sessions Current websocket sessions\n");
        out.push_str("# TYPE restsend_online_sessions gauge\n");
        out.push_str(&format!("restsend_online_sessions {}\n", snapshot.0));
        out.push_str("# HELP restsend_online_users Current online users\n");
        out.push_str("# TYPE restsend_online_users gauge\n");
        out.push_str(&format!("restsend_online_users {}\n", snapshot.1));
        for (metric, value) in daily {
            let name = format!(
                "restsend_{}",
                metric
                    .replace(['.', '-'], "_")
                    .replace("bytes", "bytes_total")
            );
            out.push_str(&format!("# TYPE {name} counter\n"));
            out.push_str(&format!("{name} {value}\n"));
        }
        out
    }

    pub fn realtime_snapshot(&self) -> (u64, u64) {
        (
            self.realtime.online_sessions.load(Ordering::Relaxed),
            self.realtime.online_users.load(Ordering::Relaxed),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn memory_service(enabled: bool) -> StatsService {
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        StatsService::new(db, enabled)
    }

    #[tokio::test]
    async fn record_accumulates_per_metric() {
        let service = memory_service(true).await;
        service.record_event(METRIC_CHAT);
        service.record_event(METRIC_CHAT);
        service.record(METRIC_UPLOAD_BYTES, 1024);
        let counters = service.daily.counters.lock().unwrap();
        let today = Utc::now().format("%Y-%m-%d").to_string();
        assert_eq!(
            counters.get(&(today.clone(), METRIC_CHAT.to_string())),
            Some(&2)
        );
        assert_eq!(
            counters
                .get(&(today, METRIC_UPLOAD_BYTES.to_string())),
            Some(&1024)
        );
    }

    #[tokio::test]
    async fn disabled_stats_records_nothing() {
        let service = memory_service(false).await;
        service.record_event(METRIC_CHAT);
        let counters = service.daily.counters.lock().unwrap();
        assert!(counters.is_empty());
    }

    #[tokio::test]
    async fn prometheus_renders_metrics() {
        let service = memory_service(true).await;
        service.record_event(METRIC_CHAT);
        let text = service.render_prometheus();
        assert!(text.contains("restsend_online_sessions"));
        assert!(text.contains("restsend_chat"));
    }
}
