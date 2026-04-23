use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    IntoActiveModel, QueryFilter,
};

use crate::entity::relation;
use crate::services::{DomainError, DomainResult};
use crate::{OpenApiRelationEditForm, Relation};

#[derive(Clone)]
pub struct RelationService {
    db: DatabaseConnection,
}

impl RelationService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn update_relation(
        &self,
        owner_id: &str,
        target_id: &str,
        form: OpenApiRelationEditForm,
    ) -> DomainResult<Relation> {
        if owner_id.trim().is_empty() || target_id.trim().is_empty() {
            return Err(DomainError::Validation(
                "owner_id and target_id are required".to_string(),
            ));
        }

        let now = Utc::now().to_rfc3339();
        let existing = relation::Entity::find_by_id((owner_id.to_string(), target_id.to_string()))
            .one(&self.db)
            .await?;

        let updated = if let Some(existing) = existing {
            let mut active = existing.into_active_model();
            if let Some(v) = form.is_contact {
                active.is_contact = Set(v);
            }
            if let Some(v) = form.is_star {
                active.is_star = Set(v);
            }
            if let Some(v) = form.is_blocked {
                active.is_blocked = Set(v);
            }
            if let Some(v) = form.remark {
                active.remark = Set(v);
            }
            if !form.source.is_empty() {
                active.source = Set(form.source);
            }
            active.updated_at = Set(now);
            active.update(&self.db).await?
        } else {
            let rel = Relation {
                owner_id: owner_id.to_string(),
                target_id: target_id.to_string(),
                is_contact: form.is_contact.unwrap_or(false),
                is_star: form.is_star.unwrap_or(false),
                is_blocked: form.is_blocked.unwrap_or(false),
                remark: form.remark.unwrap_or_default(),
                source: form.source,
            };
            let active: relation::ActiveModel = (rel, now.as_str()).into();
            active.insert(&self.db).await?
        };

        Ok(updated.into())
    }

    pub async fn list_blocked(&self, owner_id: &str) -> DomainResult<Vec<String>> {
        let rows = relation::Entity::find()
            .filter(relation::Column::OwnerId.eq(owner_id.to_string()))
            .filter(relation::Column::IsBlocked.eq(true))
            .all(&self.db)
            .await?;

        Ok(rows.into_iter().map(|r| r.target_id).collect())
    }

    pub async fn update_blocked(
        &self,
        owner_id: &str,
        user_ids: &[String],
        blocked: bool,
    ) -> DomainResult<Vec<String>> {
        let mut done = Vec::with_capacity(user_ids.len());
        for user_id in user_ids {
            let form = OpenApiRelationEditForm {
                is_blocked: Some(blocked),
                source: "openapi".to_string(),
                ..OpenApiRelationEditForm::default()
            };
            let _ = self.update_relation(owner_id, user_id, form).await?;
            done.push(user_id.clone());
        }
        Ok(done)
    }
}
