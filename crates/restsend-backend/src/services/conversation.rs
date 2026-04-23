use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    IntoActiveModel, QueryFilter, QueryOrder, QuerySelect,
};

use crate::entity::conversation;
use crate::services::{DomainError, DomainResult};
use crate::{Conversation, OpenApiUpdateConversationForm};

#[derive(Clone)]
pub struct ConversationService {
    db: DatabaseConnection,
}

impl ConversationService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn get_conversation(
        &self,
        owner_id: &str,
        topic_id: &str,
    ) -> DomainResult<Conversation> {
        let model = conversation::Entity::find_by_id((owner_id.to_string(), topic_id.to_string()))
            .one(&self.db)
            .await?
            .ok_or(DomainError::NotFound)?;
        Ok(model.into())
    }

    pub async fn create_or_update(&self, conversation: Conversation) -> DomainResult<Conversation> {
        if conversation.owner_id.trim().is_empty() || conversation.topic_id.trim().is_empty() {
            return Err(DomainError::Validation(
                "owner_id and topic_id are required".to_string(),
            ));
        }

        let now = now();
        if let Some(existing) = conversation::Entity::find_by_id((
            conversation.owner_id.clone(),
            conversation.topic_id.clone(),
        ))
        .one(&self.db)
        .await?
        {
            let mut active = existing.into_active_model();
            active.updated_at = Set(now.clone());
            active.sticky = Set(conversation.sticky);
            active.mute = Set(conversation.mute);
            active.remark = Set(conversation.remark.clone());
            active.unread = Set(conversation.unread);
            active.start_seq = Set(conversation.start_seq);
            active.last_seq = Set(conversation.last_seq);
            active.last_read_seq = Set(conversation.last_read_seq);
            active.last_read_at = Set(conversation.last_read_at.clone());
            active.multiple = Set(conversation.multiple);
            active.attendee = Set(conversation.attendee.clone());
            active.members = Set(conversation.members);
            active.name = Set(conversation.name.clone());
            active.icon = Set(conversation.icon.clone());
            active.kind = Set(conversation.kind.clone());
            active.source = Set(conversation.source.clone());
            active.last_sender_id = Set(conversation.last_sender_id.clone());
            active.last_message_json = Set(conversation
                .last_message
                .as_ref()
                .map(crate::entity::encode_json)
                .unwrap_or_else(|| "{}".to_string()));
            active.last_message_at = Set(conversation.last_message_at.clone());
            active.last_message_seq = Set(conversation.last_message_seq.unwrap_or_default());

            let updated = active.update(&self.db).await?;
            return Ok(updated.into());
        }

        let active: conversation::ActiveModel = (conversation, now.as_str()).into();
        let created = active.insert(&self.db).await?;
        Ok(created.into())
    }

    pub async fn update_conversation(
        &self,
        owner_id: &str,
        topic_id: &str,
        form: OpenApiUpdateConversationForm,
    ) -> DomainResult<Conversation> {
        let existing =
            conversation::Entity::find_by_id((owner_id.to_string(), topic_id.to_string()))
                .one(&self.db)
                .await?
                .ok_or(DomainError::NotFound)?;

        let mut active = existing.into_active_model();
        if let Some(sticky) = form.sticky {
            active.sticky = Set(sticky);
        }
        if let Some(mute) = form.mute {
            active.mute = Set(mute);
        }
        if let Some(remark) = form.remark {
            active.remark = Set(Some(remark));
        }
        active.updated_at = Set(now());

        let updated = active.update(&self.db).await?;
        Ok(updated.into())
    }

    pub async fn mark_unread(&self, owner_id: &str, topic_id: &str) -> DomainResult<Conversation> {
        let existing =
            conversation::Entity::find_by_id((owner_id.to_string(), topic_id.to_string()))
                .one(&self.db)
                .await?
                .ok_or(DomainError::NotFound)?;

        let mut active = existing.into_active_model();
        active.unread = Set(1);
        active.updated_at = Set(now());
        let updated = active.update(&self.db).await?;
        Ok(updated.into())
    }

    pub async fn clear_messages(
        &self,
        owner_id: &str,
        topic_id: &str,
        last_seq: i64,
    ) -> DomainResult<Conversation> {
        let current = self
            .get_conversation(owner_id, topic_id)
            .await
            .unwrap_or_else(|_| Conversation {
                owner_id: owner_id.to_string(),
                topic_id: topic_id.to_string(),
                ..Conversation::default()
            });

        self.create_or_update(Conversation {
            start_seq: last_seq,
            unread: 0,
            last_message: None,
            last_message_at: String::new(),
            last_message_seq: Some(0),
            last_sender_id: String::new(),
            updated_at: now(),
            ..current
        })
        .await
    }

    pub async fn mark_read(
        &self,
        owner_id: &str,
        topic_id: &str,
        last_read_seq: Option<i64>,
    ) -> DomainResult<Conversation> {
        let existing =
            conversation::Entity::find_by_id((owner_id.to_string(), topic_id.to_string()))
                .one(&self.db)
                .await?
                .ok_or(DomainError::NotFound)?;

        let fallback_seq = existing.last_seq;
        let mut active = existing.into_active_model();
        let read_seq = last_read_seq.unwrap_or(fallback_seq);
        active.last_read_seq = Set(read_seq);
        active.unread = Set(0);
        active.updated_at = Set(now());
        let updated = active.update(&self.db).await?;
        Ok(updated.into())
    }

    pub async fn mark_all_read(&self, owner_id: &str) -> DomainResult<u64> {
        let rows = conversation::Entity::find()
            .filter(conversation::Column::OwnerId.eq(owner_id.to_string()))
            .all(&self.db)
            .await?;

        let mut changed = 0;
        for row in rows {
            let row_last_seq = row.last_seq;
            let mut active = row.into_active_model();
            active.unread = Set(0);
            active.last_read_seq = Set(row_last_seq);
            active.updated_at = Set(now());
            let _ = active.update(&self.db).await?;
            changed += 1;
        }

        Ok(changed)
    }

    pub async fn list_by_user(
        &self,
        owner_id: &str,
        offset: u64,
        limit: u64,
    ) -> DomainResult<Vec<Conversation>> {
        let st = std::time::Instant::now();
        let rows: Vec<conversation::Model> = conversation::Entity::find()
            .filter(conversation::Column::OwnerId.eq(owner_id.to_string()))
            .order_by_desc(conversation::Column::UpdatedAt)
            .offset(offset)
            .limit(limit)
            .all(&self.db)
            .await?;

        let result: Vec<Conversation> = rows.into_iter().map(Conversation::from).collect();
        tracing::info!(
            owner_id = %owner_id,
            offset = offset,
            limit = limit,
            count = result.len(),
            elapsed_ms = st.elapsed().as_millis() as u64,
            "conversation list by user"
        );
        Ok(result)
    }

    pub async fn remove_conversation(&self, owner_id: &str, topic_id: &str) -> DomainResult<()> {
        let result =
            conversation::Entity::delete_by_id((owner_id.to_string(), topic_id.to_string()))
                .exec(&self.db)
                .await?;
        if result.rows_affected == 0 {
            return Err(DomainError::NotFound);
        }
        Ok(())
    }
}

fn now() -> String {
    Utc::now().to_rfc3339()
}
