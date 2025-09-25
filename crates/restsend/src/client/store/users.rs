use std::collections::HashMap;

use super::{is_cache_expired, ClientStore};
use crate::services::user::{get_users, set_user_block, set_user_remark, set_user_star};
use crate::storage::Storage;
use crate::utils::spawn_task;
use crate::{models::User, services::user::get_user, utils::now_millis};
use crate::{Result, USER_CACHE_EXPIRE_SECS};
use log::warn;

impl ClientStore {
    pub async fn set_user_remark(&self, user_id: &str, remark: &str) -> Result<()> {
        {
            let t = self.message_storage.table::<User>().await;
            if let Some(mut u) = t.get("", user_id).await {
                u.remark = remark.to_string();
                t.set("", user_id, Some(&u)).await.ok();
            }
        }

        set_user_remark(&self.endpoint, &self.token, &user_id, &remark).await
    }

    pub async fn set_user_star(&self, user_id: &str, star: bool) -> Result<()> {
        {
            let t = self.message_storage.table::<User>().await;
            if let Some(mut u) = t.get("", user_id).await {
                u.is_star = star;
                t.set("", user_id, Some(&u)).await.ok();
            }
        }

        set_user_star(&self.endpoint, &self.token, &user_id, star).await
    }

    pub async fn set_user_block(&self, user_id: &str, block: bool) -> Result<()> {
        {
            let t = self.message_storage.table::<User>().await;
            if let Some(mut u) = t.get("", user_id).await {
                u.is_blocked = block;
                t.set("", user_id, Some(&u)).await.ok();
            }
        }
        set_user_block(&self.endpoint, &self.token, &user_id, block).await
    }

    pub async fn get_user(&self, user_id: &str, blocking: bool) -> Option<User> {
        let u = {
            let t = self.message_storage.table::<User>().await;
            t.get("", user_id).await.unwrap_or(User::new(user_id))
        };

        if !(u.is_partial || is_cache_expired(u.cached_at, USER_CACHE_EXPIRE_SECS)) {
            return Some(u);
        }

        let endpoint = self.endpoint.clone();
        let token = self.token.clone();
        let user_id = user_id.to_string();
        let message_storage = self.message_storage.clone();
        let runner = async move {
            match get_user(&endpoint, &token, &user_id).await {
                Ok(user) => update_user_with_storage(&message_storage, user).await.ok(),
                Err(e) => {
                    warn!("get_user failed user_id:{} error:{:?}", user_id, e);
                    None
                }
            }
        };

        if blocking {
            Some(runner.await.unwrap_or(u))
        } else {
            spawn_task(async {
                runner.await;
            });
            Some(u)
        }
    }

    pub async fn get_users(&self, user_ids: Vec<String>) -> Vec<User> {
        let mut missing_ids = vec![];
        let mut pending_users = HashMap::new();
        {
            let t = self.message_storage.table::<User>().await;
            for user_id in user_ids {
                let u = t.get("", &user_id).await.unwrap_or(User::new(&user_id));
                if u.is_partial || is_cache_expired(u.cached_at, USER_CACHE_EXPIRE_SECS) {
                    missing_ids.push(user_id.clone());
                }
                pending_users.insert(user_id, u);
            }
        }

        match get_users(&self.endpoint, &self.token, missing_ids).await {
            Ok(us) => {
                for u in us {
                    pending_users.insert(u.user_id.clone(), u);
                }
            }
            Err(e) => {
                warn!("get_users failed: {:?}", e);
            }
        }
        pending_users.values().cloned().collect()
    }

    pub async fn fetch_user(&self, user_id: &str) -> Result<User> {
        let u = {
            let t = self.message_storage.table::<User>().await;
            t.get("", user_id).await.unwrap_or(User::new(user_id))
        };

        if u.is_partial || is_cache_expired(u.cached_at, USER_CACHE_EXPIRE_SECS) {
            let user = get_user(&self.endpoint, &self.token, &user_id).await?;
            return self.update_user(user).await;
        }
        Ok(u)
    }

    pub async fn fetch_users(&self, user_ids: Vec<String>) -> Result<Vec<User>> {
        let mut users = vec![];
        let mut missing_ids = vec![];
        for user_id in user_ids {
            let u = self.fetch_user(&user_id).await?;
            if u.is_partial || is_cache_expired(u.cached_at, USER_CACHE_EXPIRE_SECS) {
                missing_ids.push(user_id);
            } else {
                users.push(u);
            }
        }
        match get_users(&self.endpoint, &self.token, missing_ids).await {
            Ok(mut us) => {
                users.append(&mut us);
            }
            Err(e) => {
                warn!("get_users failed: {:?}", e);
            }
        }
        Ok(users)
    }

    pub(super) async fn update_user(&self, user: User) -> Result<User> {
        update_user_with_storage(&self.message_storage, user).await
    }
}

pub(super) async fn update_user_with_storage(storage: &Storage, mut user: User) -> Result<User> {
    let t = storage.table::<User>().await;

    let user_id = user.user_id.clone();
    if let Some(old_user) = t.get("", &user_id).await {
        user = old_user.merge(&user);
    }

    user.is_partial = false;
    user.cached_at = now_millis();

    let t = storage.table::<User>().await;
    t.set("", &user_id, Some(&user)).await.ok();
    Ok(user)
}
