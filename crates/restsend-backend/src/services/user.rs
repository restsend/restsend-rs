use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};

use crate::entity::user;
use crate::services::{DomainError, DomainResult};
use crate::{OpenApiUserForm, User};

#[derive(Clone)]
pub struct UserService {
    db: DatabaseConnection,
}

impl UserService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn get_by_user_id(&self, user_id: &str) -> DomainResult<User> {
        let model = user::Entity::find_by_id(user_id.to_string())
            .one(&self.db)
            .await?
            .ok_or(DomainError::NotFound)?;
        if !model.enabled {
            return Err(DomainError::Forbidden);
        }
        Ok(model.into())
    }

    pub async fn get_any_by_user_id(&self, user_id: &str) -> DomainResult<User> {
        let model = user::Entity::find_by_id(user_id.to_string())
            .one(&self.db)
            .await?
            .ok_or(DomainError::NotFound)?;
        Ok(model.into())
    }

    pub async fn get_or_create_for_auth(
        &self,
        user_id: &str,
        create_when_not_exist: bool,
    ) -> DomainResult<User> {
        match self.get_by_user_id(user_id).await {
            Ok(user) => Ok(user),
            Err(DomainError::NotFound) if create_when_not_exist => {
                self.register(user_id, OpenApiUserForm::default()).await
            }
            Err(err) => Err(err),
        }
    }

    pub async fn register(&self, user_id: &str, form: OpenApiUserForm) -> DomainResult<User> {
        if user_id.trim().is_empty() {
            return Err(DomainError::Validation("user id is required".to_string()));
        }

        if user::Entity::find_by_id(user_id.to_string())
            .one(&self.db)
            .await?
            .is_some()
        {
            return Err(DomainError::Conflict);
        }

        let now = now();
        let domain = User {
            user_id: user_id.to_string(),
            name: form.display_name,
            avatar: form.avatar,
            source: form.source,
            locale: form.locale,
            city: form.city,
            country: form.country,
            gender: form.gender,
            public_key: form.public_key,
            enabled: true,
            created_at: now.clone(),
            ..User::default()
        };

        let active: user::ActiveModel = (domain, now.as_str()).into();
        let created = active.insert(&self.db).await?;
        Ok(created.into())
    }

    pub async fn update(&self, user_id: &str, form: OpenApiUserForm) -> DomainResult<User> {
        let existing = user::Entity::find_by_id(user_id.to_string())
            .one(&self.db)
            .await?
            .ok_or(DomainError::NotFound)?;

        let mut active = existing.into_active_model();
        if !form.display_name.is_empty() {
            active.display_name = Set(form.display_name);
        }
        if !form.avatar.is_empty() {
            active.avatar = Set(form.avatar);
        }
        if !form.source.is_empty() {
            active.source = Set(form.source);
        }
        if !form.locale.is_empty() {
            active.locale = Set(form.locale);
        }
        if !form.city.is_empty() {
            active.city = Set(form.city);
        }
        if !form.country.is_empty() {
            active.country = Set(form.country);
        }
        if !form.gender.is_empty() {
            active.gender = Set(form.gender);
        }
        if !form.public_key.is_empty() {
            active.public_key = Set(form.public_key);
        }
        if !form.password.is_empty() {
            active.password = Set(crate::api::auth::hash_password(&form.password));
        }
        active.updated_at = Set(now());

        let updated = active.update(&self.db).await?;
        Ok(updated.into())
    }

    pub async fn deactive(&self, user_id: &str) -> DomainResult<()> {
        let rows = user::Entity::delete_by_id(user_id.to_string())
            .exec(&self.db)
            .await?
            .rows_affected;
        if rows == 0 {
            return Err(DomainError::NotFound);
        }
        Ok(())
    }

    pub async fn set_enabled(&self, user_id: &str, enabled: bool) -> DomainResult<User> {
        let existing = user::Entity::find_by_id(user_id.to_string())
            .one(&self.db)
            .await?
            .ok_or(DomainError::NotFound)?;
        if existing.is_staff && !enabled {
            return Err(DomainError::Validation(
                "cannot disable superuser account".to_string(),
            ));
        }
        let mut active = existing.into_active_model();
        active.enabled = Set(enabled);
        active.updated_at = Set(now());
        let updated = active.update(&self.db).await?;
        Ok(updated.into())
    }

    pub async fn set_staff(&self, user_id: &str, is_staff: bool) -> DomainResult<User> {
        let existing = user::Entity::find_by_id(user_id.to_string())
            .one(&self.db)
            .await?
            .ok_or(DomainError::NotFound)?;
        let mut active = existing.into_active_model();
        active.is_staff = Set(is_staff);
        active.updated_at = Set(now());
        let updated = active.update(&self.db).await?;
        Ok(updated.into())
    }

    pub async fn list_users(
        &self,
        offset: u64,
        limit: u64,
        keyword: Option<&str>,
    ) -> DomainResult<(Vec<User>, u64)> {
        let limit = limit.clamp(1, 200);
        let mut query = user::Entity::find().order_by_asc(user::Column::UserId);
        if let Some(keyword) = keyword.map(str::trim).filter(|v| !v.is_empty()) {
            query = query.filter(
                user::Column::UserId
                    .contains(keyword)
                    .or(user::Column::DisplayName.contains(keyword)),
            );
        }
        let total = query.clone().count(&self.db).await?;
        let rows: Vec<user::Model> = query.offset(offset).limit(limit).all(&self.db).await?;
        Ok((rows.into_iter().map(Into::into).collect(), total))
    }
}

fn now() -> String {
    Utc::now().to_rfc3339()
}
