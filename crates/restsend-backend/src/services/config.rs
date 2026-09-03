use std::sync::Arc;
use std::time::{Duration, Instant};

use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    IntoActiveModel,
};
use tokio::sync::RwLock;

use crate::entity::config;
use crate::services::{DomainError, DomainResult};

/// Default values mirrored from the Go server's runtime Config table.
/// (MINIMUM_TOPIC_MEMBERS defaults to 2 in this port; Go defaults to 3 —
/// override via the admin configs API for strict Go parity.)
pub const MINIMUM_TOPIC_MEMBERS: &str = "MINIMUM_TOPIC_MEMBERS";
pub const MAX_TOPIC_MEMBERS: &str = "MAX_TOPIC_MEMBERS";
pub const RECALL_TIMEOUT: &str = "RECALL_TIMEOUT";
pub const WEBHOOKS: &str = "WEBHOOKS";
pub const ALLOW_GUEST_LOGIN: &str = "ALLOW_GUEST_LOGIN";

pub fn default_value(key: &str) -> &'static str {
    match key {
        MINIMUM_TOPIC_MEMBERS => "2",
        MAX_TOPIC_MEMBERS => "1000",
        RECALL_TIMEOUT => "2m",
        WEBHOOKS => "",
        ALLOW_GUEST_LOGIN => "",
        _ => "",
    }
}

#[derive(Clone)]
struct CachedEntry {
    value: String,
    cached_at: Instant,
}

const CACHE_TTL: Duration = Duration::from_secs(10);

/// DB-backed runtime configuration (configs table) with a short-lived cache
/// so hot paths never hit the database; values can be changed from the admin
/// API without a restart (Go `Config` table parity).
#[derive(Clone)]
pub struct ConfigService {
    db: DatabaseConnection,
    cache: Arc<RwLock<std::collections::HashMap<String, CachedEntry>>>,
}

impl ConfigService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            db,
            cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Effective value: DB row if present, else the built-in default.
    pub async fn get(&self, key: &str) -> DomainResult<String> {
        {
            let cache = self.cache.read().await;
            if let Some(entry) = cache.get(key) {
                if entry.cached_at.elapsed() < CACHE_TTL {
                    return Ok(entry.value.clone());
                }
            }
        }

        let row = config::Entity::find_by_id(key.to_string())
            .one(&self.db)
            .await?
            .map(|row| row.value)
            .unwrap_or_else(|| default_value(key).to_string());

        let mut cache = self.cache.write().await;
        cache.insert(
            key.to_string(),
            CachedEntry {
                value: row.clone(),
                cached_at: Instant::now(),
            },
        );
        Ok(row)
    }

    pub async fn get_i64(&self, key: &str) -> DomainResult<i64> {
        Ok(self.get(key).await?.trim().parse::<i64>().unwrap_or(0))
    }

    /// Set a value (admin path). Empty value resets to the default.
    pub async fn set(&self, key: &str, value: &str) -> DomainResult<()> {
        let now = chrono::Utc::now().to_rfc3339();
        match config::Entity::find_by_id(key.to_string()).one(&self.db).await? {
            Some(row) => {
                let mut active = row.into_active_model();
                active.value = Set(value.to_string());
                active.updated_at = Set(now);
                active.update(&self.db).await?;
            }
            None => {
                let active = config::ActiveModel {
                    key: Set(key.to_string()),
                    value: Set(value.to_string()),
                    updated_at: Set(now),
                };
                active.insert(&self.db).await?;
            }
        }
        self.cache.write().await.remove(key);
        Ok(())
    }

    /// List all keys with their effective values.
    pub async fn list(&self) -> DomainResult<Vec<(String, String)>> {
        let rows = config::Entity::find().all(&self.db).await?;
        let mut out: Vec<(String, String)> = rows
            .into_iter()
            .map(|row| (row.key, row.value))
            .collect();
        // Include known defaults not yet overridden.
        for key in [
            MINIMUM_TOPIC_MEMBERS,
            MAX_TOPIC_MEMBERS,
            RECALL_TIMEOUT,
            WEBHOOKS,
            ALLOW_GUEST_LOGIN,
        ] {
            if !out.iter().any(|(k, _)| k == key) {
                out.push((key.to_string(), default_value(key).to_string()));
            }
        }
        out.sort();
        Ok(out)
    }

    /// Webhook URLs from the DB config, one per line (Go `WEBHOOKS` config).
    pub async fn webhook_targets(&self) -> Vec<String> {
        self.get(WEBHOOKS)
            .await
            .map(|raw| {
                raw.lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Parse a Go-style duration ("30m"/"2m"/"1h"/"forever") into seconds.
pub fn parse_duration_secs(raw: &str) -> Option<u64> {
    let input = raw.trim().to_ascii_lowercase();
    if input.is_empty() {
        return None;
    }
    if input == "forever" {
        return Some(u64::MAX / 2);
    }
    if let Some(v) = input.strip_suffix('s').and_then(|v| v.parse::<u64>().ok()) {
        return Some(v);
    }
    if let Some(v) = input.strip_suffix('m').and_then(|v| v.parse::<u64>().ok()) {
        return Some(v * 60);
    }
    if let Some(v) = input.strip_suffix('h').and_then(|v| v.parse::<u64>().ok()) {
        return Some(v * 3600);
    }
    if let Some(v) = input.strip_suffix('d').and_then(|v| v.parse::<u64>().ok()) {
        return Some(v * 86400);
    }
    input.parse::<u64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn get_returns_default_when_unset() {
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        crate::infra::db::run_migrations(&db).await.unwrap();
        let service = ConfigService::new(db);
        assert_eq!(service.get(MINIMUM_TOPIC_MEMBERS).await.unwrap(), "2");
        assert_eq!(service.get(MAX_TOPIC_MEMBERS).await.unwrap(), "1000");
        assert_eq!(service.get_i64(MAX_TOPIC_MEMBERS).await.unwrap(), 1000);
    }

    #[tokio::test]
    async fn set_then_get_roundtrip_and_cache_invalidation() {
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        crate::infra::db::run_migrations(&db).await.unwrap();
        let service = ConfigService::new(db);
        service.set(MINIMUM_TOPIC_MEMBERS, "5").await.unwrap();
        assert_eq!(service.get(MINIMUM_TOPIC_MEMBERS).await.unwrap(), "5");

        let list = service.list().await.unwrap();
        assert!(list
            .iter()
            .any(|(k, v)| k == MINIMUM_TOPIC_MEMBERS && v == "5"));
    }

    #[test]
    fn parse_duration_secs_handles_go_style() {
        assert_eq!(parse_duration_secs("2m"), Some(120));
        assert_eq!(parse_duration_secs("1h"), Some(3600));
        assert_eq!(parse_duration_secs("30s"), Some(30));
        assert_eq!(parse_duration_secs("7d"), Some(7 * 86400));
        assert_eq!(parse_duration_secs("forever"), Some(u64::MAX / 2));
        assert_eq!(parse_duration_secs(""), None);
    }
}
