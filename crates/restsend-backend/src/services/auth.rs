use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    IntoActiveModel, QueryFilter,
};

use crate::entity::{auth_token, user};
use crate::services::DomainResult;

#[derive(Clone)]
pub struct AuthService {
    db: DatabaseConnection,
}

impl AuthService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn issue_token(&self, user_id: &str) -> DomainResult<String> {
        if let Some(model) = user::Entity::find_by_id(user_id.to_string())
            .one(&self.db)
            .await?
        {
            if !model.enabled {
                return Err(crate::services::DomainError::Forbidden);
            }
        }
        let now = Utc::now().to_rfc3339();
        let token = format!("rs_{}_{}", user_id, uuid::Uuid::new_v4());
        auth_token::ActiveModel {
            token: Set(token.clone()),
            user_id: Set(user_id.to_string()),
            created_at: Set(now.clone()),
            last_seen_at: Set(now),
        }
        .insert(&self.db)
        .await?;
        Ok(token)
    }

    pub async fn validate(&self, token: &str) -> DomainResult<Option<String>> {
        let found = auth_token::Entity::find_by_id(token.to_string())
            .one(&self.db)
            .await?;

        if let Some(model) = found {
            if let Some(user) = user::Entity::find_by_id(model.user_id.clone())
                .one(&self.db)
                .await?
            {
                if !user.enabled {
                    return Ok(None);
                }
            }
            let mut active = model.into_active_model();
            active.last_seen_at = Set(Utc::now().to_rfc3339());
            let updated = active.update(&self.db).await?;
            Ok(Some(updated.user_id))
        } else {
            Ok(None)
        }
    }

    pub async fn revoke_by_user(&self, user_id: &str) -> DomainResult<u64> {
        let result = auth_token::Entity::delete_many()
            .filter(auth_token::Column::UserId.eq(user_id.to_string()))
            .exec(&self.db)
            .await?;
        Ok(result.rows_affected)
    }

    pub async fn revoke_token(&self, token: &str) -> DomainResult<bool> {
        let rows = auth_token::Entity::delete_by_id(token.to_string())
            .exec(&self.db)
            .await?
            .rows_affected;
        Ok(rows > 0)
    }
}
