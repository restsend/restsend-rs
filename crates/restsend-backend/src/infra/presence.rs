use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    IntoActiveModel, PaginatorTrait, QueryFilter,
};
use tokio::sync::RwLock;

use crate::entity::presence_session;

#[derive(Clone, Debug, Default)]
pub struct PresenceSnapshot {
    pub online: bool,
    pub devices: Vec<String>,
}

#[async_trait]
pub trait PresenceStore: Send + Sync {
    async fn upsert_session(&self, user_id: &str, device: &str) -> Result<(), sea_orm::DbErr>;
    async fn remove_session(&self, user_id: &str, device: &str) -> Result<(), sea_orm::DbErr>;
    async fn list_devices(&self, user_id: &str) -> Result<Vec<String>, sea_orm::DbErr>;
    async fn is_online(&self, user_id: &str) -> Result<bool, sea_orm::DbErr>;
    async fn cleanup_expired(&self) -> Result<u64, sea_orm::DbErr>;
}

#[derive(Clone)]
pub struct MemoryPresenceStore {
    sessions: Arc<RwLock<HashMap<String, HashMap<String, i64>>>>,
    ttl_secs: u64,
}

impl MemoryPresenceStore {
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            ttl_secs,
        }
    }

    fn now_unix() -> i64 {
        chrono::Utc::now().timestamp()
    }

    fn expired(cutoff: i64, updated_at_unix: i64) -> bool {
        updated_at_unix < cutoff
    }

    fn cutoff(&self) -> i64 {
        Self::now_unix() - self.ttl_secs as i64
    }
}

#[async_trait]
impl PresenceStore for MemoryPresenceStore {
    async fn upsert_session(&self, user_id: &str, device: &str) -> Result<(), sea_orm::DbErr> {
        let mut guard = self.sessions.write().await;
        let entry = guard.entry(user_id.to_string()).or_default();
        entry.insert(device.to_string(), Self::now_unix());
        Ok(())
    }

    async fn remove_session(&self, user_id: &str, device: &str) -> Result<(), sea_orm::DbErr> {
        let mut guard = self.sessions.write().await;
        if let Some(devices) = guard.get_mut(user_id) {
            devices.remove(device);
            if devices.is_empty() {
                guard.remove(user_id);
            }
        }
        Ok(())
    }

    async fn list_devices(&self, user_id: &str) -> Result<Vec<String>, sea_orm::DbErr> {
        let mut guard = self.sessions.write().await;
        let cutoff = self.cutoff();
        let devices = guard.get_mut(user_id).map(|m| {
            m.retain(|_, updated| !Self::expired(cutoff, *updated));
            m.keys().cloned().collect::<Vec<_>>()
        });
        Ok(devices.unwrap_or_default())
    }

    async fn is_online(&self, user_id: &str) -> Result<bool, sea_orm::DbErr> {
        Ok(!self.list_devices(user_id).await?.is_empty())
    }

    async fn cleanup_expired(&self) -> Result<u64, sea_orm::DbErr> {
        let mut guard = self.sessions.write().await;
        let cutoff = self.cutoff();
        let mut removed = 0u64;
        guard.retain(|_, devices| {
            let before = devices.len();
            devices.retain(|_, updated| !Self::expired(cutoff, *updated));
            removed += before.saturating_sub(devices.len()) as u64;
            !devices.is_empty()
        });
        Ok(removed)
    }
}

#[derive(Clone)]
pub struct DbPresenceStore {
    db: DatabaseConnection,
    node_id: String,
    endpoint: String,
    ttl_secs: u64,
}

impl DbPresenceStore {
    pub fn new(db: DatabaseConnection, node_id: String, endpoint: String, ttl_secs: u64) -> Self {
        Self {
            db,
            node_id,
            endpoint,
            ttl_secs,
        }
    }

    fn now_unix() -> i64 {
        chrono::Utc::now().timestamp()
    }

    fn cutoff(&self) -> i64 {
        Self::now_unix() - self.ttl_secs as i64
    }
}

#[async_trait]
impl PresenceStore for DbPresenceStore {
    async fn upsert_session(&self, user_id: &str, device: &str) -> Result<(), sea_orm::DbErr> {
        let now = Self::now_unix();
        if let Some(existing) =
            presence_session::Entity::find_by_id((user_id.to_string(), device.to_string()))
                .one(&self.db)
                .await?
        {
            let mut active = existing.into_active_model();
            active.node_id = Set(self.node_id.clone());
            active.endpoint = Set(self.endpoint.clone());
            active.updated_at_unix = Set(now);
            let _ = active.update(&self.db).await?;
            return Ok(());
        }

        let model = presence_session::ActiveModel {
            user_id: Set(user_id.to_string()),
            device: Set(device.to_string()),
            node_id: Set(self.node_id.clone()),
            endpoint: Set(self.endpoint.clone()),
            updated_at_unix: Set(now),
        };
        let _ = model.insert(&self.db).await?;
        Ok(())
    }

    async fn remove_session(&self, user_id: &str, device: &str) -> Result<(), sea_orm::DbErr> {
        let _ = presence_session::Entity::delete_by_id((user_id.to_string(), device.to_string()))
            .exec(&self.db)
            .await?;
        Ok(())
    }

    async fn list_devices(&self, user_id: &str) -> Result<Vec<String>, sea_orm::DbErr> {
        let rows = presence_session::Entity::find()
            .filter(presence_session::Column::UserId.eq(user_id.to_string()))
            .filter(presence_session::Column::UpdatedAtUnix.gte(self.cutoff()))
            .all(&self.db)
            .await?;
        Ok(rows.into_iter().map(|m| m.device).collect())
    }

    async fn is_online(&self, user_id: &str) -> Result<bool, sea_orm::DbErr> {
        let count = presence_session::Entity::find()
            .filter(presence_session::Column::UserId.eq(user_id.to_string()))
            .filter(presence_session::Column::UpdatedAtUnix.gte(self.cutoff()))
            .count(&self.db)
            .await?;
        Ok(count > 0)
    }

    async fn cleanup_expired(&self) -> Result<u64, sea_orm::DbErr> {
        let result = presence_session::Entity::delete_many()
            .filter(presence_session::Column::UpdatedAtUnix.lt(self.cutoff()))
            .exec(&self.db)
            .await?;
        Ok(result.rows_affected)
    }
}

#[derive(Clone)]
pub struct PresenceHub {
    store: Arc<dyn PresenceStore>,
}

impl PresenceHub {
    pub fn new(store: Arc<dyn PresenceStore>) -> Self {
        Self { store }
    }

    pub async fn upsert_session(&self, user_id: &str, device: &str) {
        if let Err(err) = self.store.upsert_session(user_id, device).await {
            tracing::warn!(user_id = %user_id, device = %device, error = %err, "presence upsert failed");
        }
    }

    pub async fn remove_session(&self, user_id: &str, device: &str) {
        if let Err(err) = self.store.remove_session(user_id, device).await {
            tracing::warn!(user_id = %user_id, device = %device, error = %err, "presence remove failed");
        }
    }

    pub async fn snapshot(&self, user_id: &str) -> PresenceSnapshot {
        let devices = match self.store.list_devices(user_id).await {
            Ok(v) => v,
            Err(err) => {
                tracing::warn!(user_id = %user_id, error = %err, "presence list devices failed");
                Vec::new()
            }
        };
        PresenceSnapshot {
            online: !devices.is_empty(),
            devices,
        }
    }

    pub fn start_cleanup_loop(&self, interval_secs: u64) {
        let this = self.clone();
        tokio::spawn(async move {
            let mut ticker =
                tokio::time::interval(std::time::Duration::from_secs(interval_secs.max(1)));
            loop {
                ticker.tick().await;
                match this.store.cleanup_expired().await {
                    Ok(removed) => {
                        if removed > 0 {
                            tracing::info!(
                                removed = removed,
                                "presence cleanup removed expired sessions"
                            );
                        }
                    }
                    Err(err) => tracing::warn!(error = %err, "presence cleanup failed"),
                }
            }
        });
    }
}
