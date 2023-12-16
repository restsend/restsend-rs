use super::{is_cache_expired, ClientStore};
use crate::services::user::{get_users, set_user_block, set_user_remark, set_user_star};
use crate::storage::Storage;
use crate::utils::spawn;
use crate::{models::User, services::user::get_user, utils::now_millis};
use crate::{Result, USER_CACHE_EXPIRE_SECS};
use log::warn;

impl ClientStore {
    pub async fn set_user_remark(&self, user_id: &str, remark: &str) -> Result<()> {
        {
            let t = self.message_storage.table::<User>("users");
            if let Some(mut u) = t.get("", user_id) {
                u.remark = remark.to_string();
                t.set("", user_id, Some(u));
            }
        }

        set_user_remark(&self.endpoint, &self.token, &user_id, &remark).await
    }

    pub async fn set_user_star(&self, user_id: &str, star: bool) -> Result<()> {
        {
            let t = self.message_storage.table::<User>("users");
            if let Some(mut u) = t.get("", user_id) {
                u.is_star = star;
                t.set("", user_id, Some(u));
            }
        }

        set_user_star(&self.endpoint, &self.token, &user_id, star).await
    }

    pub async fn set_user_block(&self, user_id: &str, block: bool) -> Result<()> {
        {
            let t = self.message_storage.table::<User>("users");
            if let Some(mut u) = t.get("", user_id) {
                u.is_blocked = block;
                t.set("", user_id, Some(u));
            }
        }
        set_user_block(&self.endpoint, &self.token, &user_id, block).await
    }

    pub fn get_user(&self, user_id: &str) -> Option<User> {
        let t = self.message_storage.table::<User>("users");
        let u = t.get("", user_id).unwrap_or(User::new(user_id));
        if u.is_partial || is_cache_expired(u.cached_at, USER_CACHE_EXPIRE_SECS) {
            let endpoint = self.endpoint.clone();
            let token = self.token.clone();
            let user_id = user_id.to_string();
            let message_storage = self.message_storage.clone();
            spawn(async move {
                match get_user(&endpoint, &token, &user_id).await {
                    Ok(user) => {
                        update_user_with_storage(&message_storage, user).ok();
                    }
                    Err(e) => {
                        warn!("get_user failed user_id:{} error:{:?}", user_id, e);
                    }
                }
            });
        }
        Some(u)
    }

    pub async fn fetch_user(&self, user_id: &str) -> Result<User> {
        let u = {
            let t = self.message_storage.table::<User>("users");
            t.get("", user_id).unwrap_or(User::new(user_id))
        };

        if u.is_partial || is_cache_expired(u.cached_at, USER_CACHE_EXPIRE_SECS) {
            let user = get_user(&self.endpoint, &self.token, &user_id).await?;
            return self.update_user(user);
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

    pub(super) async fn fetch_or_update_user(
        &self,
        user_id: &str,
        profile: Option<User>,
    ) -> Result<()> {
        match profile {
            Some(profile) => {
                self.update_user(profile).ok();
            }
            None => {
                let endpoint = self.endpoint.clone();
                let token = self.token.clone();
                let user_id = user_id.to_string();
                let message_storage = self.message_storage.clone();

                spawn(async move {
                    match get_user(&endpoint, &token, &user_id).await {
                        Ok(user) => {
                            update_user_with_storage(&message_storage, user).ok();
                        }
                        Err(e) => {
                            warn!("get_user failed user_id:{} error:{:?}", user_id, e);
                        }
                    }
                });
            }
        }
        Ok(())
    }

    pub(super) fn update_user(&self, user: User) -> Result<User> {
        update_user_with_storage(&self.message_storage, user)
    }
}

pub(super) fn update_user_with_storage(storage: &Storage, mut user: User) -> Result<User> {
    let t = storage.table::<User>("users");

    let user_id = user.user_id.clone();
    if let Some(old_user) = t.get("", &user_id) {
        user = old_user.merge(&user);
    }

    user.is_partial = false;
    user.cached_at = now_millis();

    t.set("", &user_id, Some(user.clone()));
    Ok(user)
}
