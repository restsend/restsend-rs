use super::ClientStore;
use crate::{
    client::store::StoreEvent, models::User, services::user::get_user, utils::now_timestamp,
};
use anyhow::Result;
use log::warn;

impl ClientStore {
    async fn fetch_user(&self, user_id: &str) {
        if let Some(event_tx) = self.event_tx.lock().unwrap().clone() {
            let endpoint = self.endpoint.clone();
            let token = self.token.clone();
            let user_id = user_id.to_string();

            tokio::spawn(async move {
                let user = get_user(&endpoint, &token, &user_id).await;
                if let Ok(user) = user {
                    event_tx.send(StoreEvent::UpdateUser(vec![user])).ok();
                } else {
                    warn!("fetch_user failed");
                }
            });
        }
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
                self.fetch_user(user_id).await;
            }
        }
        Ok(())
    }

    pub(super) fn update_user(&self, mut user: User) -> Result<User> {
        let t = self
            .message_storage
            .table::<User>("users")
            .ok_or(anyhow::anyhow!("update_user: get table failed"))?;

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
