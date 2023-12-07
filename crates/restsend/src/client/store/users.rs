use super::{is_cache_expired, ClientStore};
use crate::services::user::{set_user_block, set_user_remark, set_user_star};
use crate::{
    client::store::StoreEvent, models::User, services::user::get_user, utils::now_timestamp,
};
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
            self.fetch_user(user_id);
        }
        Some(u)
    }

    fn fetch_user(&self, user_id: &str) {
        let event_tx = self.event_tx.lock().unwrap().clone();

        let endpoint = self.endpoint.clone();
        let token = self.token.clone();
        let user_id = user_id.to_string();

        tokio::spawn(async move {
            let user = get_user(&endpoint, &token, &user_id).await;
            if let Ok(user) = user {
                event_tx.map(|tx| tx.send(StoreEvent::UpdateUser(vec![user])));
            } else {
                warn!("fetch_user failed");
            }
        });
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
                self.fetch_user(user_id);
            }
        }
        Ok(())
    }

    pub(super) fn update_user(&self, mut user: User) -> Result<User> {
        let t = self.message_storage.table::<User>("users");

        let user_id = user.user_id.clone();
        if let Some(old_user) = t.get("", &user_id) {
            user = old_user.merge(&user);
        }

        user.is_partial = false;
        user.cached_at = now_timestamp();

        t.set("", &user_id, Some(user.clone()));
        Ok(user)
    }
}
