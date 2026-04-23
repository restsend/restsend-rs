use chrono::Utc;
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    QueryOrder, QuerySelect,
};

use crate::entity::{chat_log, topic};
use crate::services::{DomainError, DomainResult};
use crate::{
    ChatLog, ChatLogSyncForm, ChatLogSyncResult, OpenApiChatMessageForm,
    OpenApiImportTopicMessageForm, OpenApiImportTopicMessageResponse, OpenApiSendMessageResponse,
};

#[derive(Clone)]
pub struct ChatService {
    db: DatabaseConnection,
}

impl ChatService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn send_to_topic(
        &self,
        topic_id: &str,
        sender_id: &str,
        form: &OpenApiChatMessageForm,
    ) -> DomainResult<OpenApiSendMessageResponse> {
        let st = std::time::Instant::now();
        if topic_id.trim().is_empty() {
            return Err(DomainError::Validation("topic id is required".to_string()));
        }
        let result = match form
            .content
            .as_ref()
            .map(|content| content.content_type.as_str())
        {
            Some("recall") => self.recall_in_topic(topic_id, sender_id, form).await,
            Some("update.extra") => self.update_extra_in_topic(topic_id, sender_id, form).await,
            _ => {
                self.send_internal(Some(topic_id.to_string()), sender_id, None, form)
                    .await
            }
        };
        tracing::info!(
            topic_id = %topic_id,
            sender_id = %sender_id,
            req_type = %form.r#type,
            elapsed_ms = st.elapsed().as_millis() as u64,
            ok = result.is_ok(),
            "chat send_to_topic"
        );
        result
    }

    pub async fn recall_in_topic(
        &self,
        topic_id: &str,
        sender_id: &str,
        form: &OpenApiChatMessageForm,
    ) -> DomainResult<OpenApiSendMessageResponse> {
        let recall_chat_id = form
            .content
            .as_ref()
            .map(|content| content.text.trim())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| DomainError::Validation("recall chat id is required".to_string()))?;

        let target = chat_log::Entity::find()
            .filter(chat_log::Column::TopicId.eq(topic_id.to_string()))
            .filter(chat_log::Column::Id.eq(recall_chat_id.to_string()))
            .filter(chat_log::Column::SenderId.eq(sender_id.to_string()))
            .one(&self.db)
            .await?
            .ok_or_else(|| DomainError::Validation("recall target not found".to_string()))?;

        if target.recall {
            return Err(DomainError::Validation(
                "recall target already recalled".to_string(),
            ));
        }

        let mut target_active = target.into_active_model();
        target_active.recall = sea_orm::ActiveValue::Set(true);
        target_active.content_json = sea_orm::ActiveValue::Set(
            serde_json::to_string(&crate::Content {
                content_type: "recalled".to_string(),
                ..crate::Content::default()
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?,
        );
        target_active.update(&self.db).await?;

        self.send_internal(Some(topic_id.to_string()), sender_id, None, form)
            .await
    }

    pub async fn update_extra_in_topic(
        &self,
        topic_id: &str,
        sender_id: &str,
        form: &OpenApiChatMessageForm,
    ) -> DomainResult<OpenApiSendMessageResponse> {
        let content = form.content.as_ref().ok_or_else(|| {
            DomainError::Validation("update extra content is required".to_string())
        })?;
        let target_chat_id = content.text.trim();
        if target_chat_id.is_empty() {
            return Err(DomainError::Validation(
                "update extra chat id is required".to_string(),
            ));
        }

        let target = chat_log::Entity::find()
            .filter(chat_log::Column::TopicId.eq(topic_id.to_string()))
            .filter(chat_log::Column::Id.eq(target_chat_id.to_string()))
            .filter(chat_log::Column::SenderId.eq(sender_id.to_string()))
            .one(&self.db)
            .await?
            .ok_or_else(|| DomainError::Validation("update extra target not found".to_string()))?;

        if target.recall {
            return Err(DomainError::Validation(
                "update extra target already recalled".to_string(),
            ));
        }

        let mut updated_content: crate::Content = crate::entity::decode_json(&target.content_json);
        updated_content.extra = content.extra.clone();

        let mut target_active = target.into_active_model();
        target_active.content_json = sea_orm::ActiveValue::Set(
            serde_json::to_string(&updated_content)
                .map_err(|e| DomainError::Validation(e.to_string()))?,
        );
        target_active.update(&self.db).await?;

        self.send_internal(Some(topic_id.to_string()), sender_id, None, form)
            .await
    }

    pub async fn send_to_user(
        &self,
        sender_id: &str,
        attendee_id: &str,
        form: &OpenApiChatMessageForm,
    ) -> DomainResult<OpenApiSendMessageResponse> {
        let st = std::time::Instant::now();
        if attendee_id.trim().is_empty() {
            return Err(DomainError::Validation(
                "attendee id is required".to_string(),
            ));
        }
        let result = self
            .send_internal(None, sender_id, Some(attendee_id.to_string()), form)
            .await;
        tracing::info!(
            sender_id = %sender_id,
            attendee_id = %attendee_id,
            req_type = %form.r#type,
            elapsed_ms = st.elapsed().as_millis() as u64,
            ok = result.is_ok(),
            "chat send_to_user"
        );
        result
    }

    async fn send_internal(
        &self,
        topic_id: Option<String>,
        sender_id: &str,
        attendee_id: Option<String>,
        form: &OpenApiChatMessageForm,
    ) -> DomainResult<OpenApiSendMessageResponse> {
        let now = Utc::now().to_rfc3339();
        let chat_id = if form.chat_id.is_empty() {
            format!("chat-{}", uuid::Uuid::new_v4().simple())
        } else {
            form.chat_id.clone()
        };
        let target_topic = topic_id.unwrap_or_else(|| {
            let attendee = attendee_id.clone().unwrap_or_default();
            if sender_id <= attendee.as_str() {
                format!("{sender_id}:{attendee}")
            } else {
                format!("{attendee}:{sender_id}")
            }
        });

        if let Some(attendee) = attendee_id.as_ref() {
            self.ensure_dm_topic(&target_topic, sender_id, attendee)
                .await?;
        }

        self.ensure_topic_enabled(&target_topic).await?;

        let seq = self.next_topic_seq(&target_topic).await?;
        let content = form.content.clone().unwrap_or_else(|| crate::Content {
            content_type: if form.r#type.is_empty() {
                "chat".to_string()
            } else {
                form.r#type.clone()
            },
            text: form.message.clone(),
            ..crate::Content::default()
        });

        let log = ChatLog {
            topic_id: target_topic.clone(),
            id: chat_id.clone(),
            seq,
            created_at: if let Some(ts) = &form.created_at {
                ts.clone()
            } else {
                now.clone()
            },
            sender_id: sender_id.to_string(),
            content,
            ..ChatLog::default()
        };

        let active: chat_log::ActiveModel = log.into();
        active.insert(&self.db).await?;

        Ok(OpenApiSendMessageResponse {
            sender_id: sender_id.to_string(),
            topic_id: target_topic,
            attendee_id: attendee_id.unwrap_or_default(),
            chat_id,
            code: 200,
            message: "ok".to_string(),
            seq,
            usage: 0,
        })
    }

    async fn next_topic_seq(&self, topic_id: &str) -> DomainResult<i64> {
        for _ in 0..5 {
            let current = topic::Entity::find_by_id(topic_id.to_string())
                .one(&self.db)
                .await?
                .ok_or(DomainError::NotFound)?;
            let current_seq = current.last_seq;
            let update = topic::Entity::update_many()
                .col_expr(
                    topic::Column::LastSeq,
                    Expr::col(topic::Column::LastSeq).add(1),
                )
                .filter(topic::Column::Id.eq(topic_id.to_string()))
                .filter(topic::Column::LastSeq.eq(current_seq))
                .exec(&self.db)
                .await?;
            if update.rows_affected > 0 {
                return Ok(current_seq + 1);
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        Err(DomainError::Conflict)
    }

    async fn ensure_dm_topic(
        &self,
        topic_id: &str,
        sender_id: &str,
        attendee_id: &str,
    ) -> DomainResult<()> {
        if topic::Entity::find_by_id(topic_id.to_string())
            .one(&self.db)
            .await?
            .is_some()
        {
            return Ok(());
        }

        let now = Utc::now().to_rfc3339();
        let row = crate::Topic {
            id: topic_id.to_string(),
            owner_id: sender_id.to_string(),
            attendee_id: attendee_id.to_string(),
            members: 2,
            multiple: false,
            name: format!("DM with {attendee_id}"),
            source: "openapi".to_string(),
            enabled: true,
            created_at: now.clone(),
            updated_at: now,
            ..crate::Topic::default()
        };
        let active: topic::ActiveModel = (row, topic_id).into();
        let _ = active.insert(&self.db).await?;
        Ok(())
    }

    async fn ensure_topic_enabled(&self, topic_id: &str) -> DomainResult<()> {
        let model = topic::Entity::find_by_id(topic_id.to_string())
            .one(&self.db)
            .await?
            .ok_or(DomainError::NotFound)?;
        if !model.enabled {
            return Err(DomainError::Forbidden);
        }
        Ok(())
    }

    pub async fn topic_logs(
        &self,
        topic_id: &str,
        form: &ChatLogSyncForm,
    ) -> DomainResult<ChatLogSyncResult> {
        let st = std::time::Instant::now();
        let mut query = chat_log::Entity::find()
            .filter(chat_log::Column::TopicId.eq(topic_id.to_string()))
            .order_by_desc(chat_log::Column::Seq);

        if let Some(last_seq) = form.last_seq {
            if last_seq > 0 {
                query = query.filter(chat_log::Column::Seq.lte(last_seq));
            }
        }

        let limit = form.limit.unwrap_or(50).clamp(1, 200);
        let rows: Vec<chat_log::Model> = query.limit(limit + 1).all(&self.db).await?;
        let has_more = rows.len() as u64 > limit;
        let items: Vec<ChatLog> = rows
            .into_iter()
            .take(limit as usize)
            .map(ChatLog::from)
            .collect();
        let last_seq = items.last().map(|v| v.seq).unwrap_or(0);
        let result = ChatLogSyncResult {
            topic_id: Some(topic_id.to_string()),
            has_more,
            updated_at: Utc::now().to_rfc3339(),
            last_seq,
            items,
        };
        tracing::info!(
            topic_id = %topic_id,
            limit = limit,
            has_more = result.has_more,
            item_count = result.items.len(),
            elapsed_ms = st.elapsed().as_millis() as u64,
            "chat topic logs sync"
        );
        Ok(result)
    }

    pub async fn remove_conversation_messages(
        &self,
        topic_id: &str,
        user_id: &str,
        chat_ids: &[String],
    ) -> DomainResult<()> {
        if chat_ids.is_empty() {
            return Ok(());
        }

        let rows = chat_log::Entity::find()
            .filter(chat_log::Column::TopicId.eq(topic_id.to_string()))
            .filter(chat_log::Column::Id.is_in(chat_ids.iter().cloned()))
            .all(&self.db)
            .await?;

        for row in rows {
            let mut active = row.into_active_model();
            let mut deleted_by: Vec<String> = serde_json::from_str(
                &active
                    .deleted_by_json
                    .clone()
                    .take()
                    .unwrap_or_else(|| "[]".to_string()),
            )
            .unwrap_or_default();
            if !deleted_by.iter().any(|v| v == user_id) {
                deleted_by.push(user_id.to_string());
            }
            active.deleted_by_json = sea_orm::ActiveValue::Set(
                serde_json::to_string(&deleted_by).unwrap_or_else(|_| "[]".to_string()),
            );
            let _ = active.update(&self.db).await?;
        }
        Ok(())
    }

    pub async fn clear_conversation_messages(&self, topic_id: &str) -> DomainResult<i64> {
        let row = topic::Entity::find_by_id(topic_id.to_string())
            .one(&self.db)
            .await?
            .ok_or(DomainError::NotFound)?;
        Ok(row.last_seq)
    }

    pub async fn import_topic_logs(
        &self,
        topic_id: &str,
        form: OpenApiImportTopicMessageForm,
    ) -> DomainResult<OpenApiImportTopicMessageResponse> {
        let mut ids = Vec::with_capacity(form.messages.len());
        for msg in form.messages {
            let seq = self.next_topic_seq(topic_id).await?;
            let chat_id = if msg.chat_id.is_empty() {
                format!("chat-{}", uuid::Uuid::new_v4().simple())
            } else {
                msg.chat_id
            };

            let mut content = msg.content.unwrap_or_default();
            if !msg.source.is_empty() {
                let mut extra = content.extra.unwrap_or_default();
                extra.insert("source".to_string(), msg.source);
                content.extra = Some(extra);
            }

            let log = ChatLog {
                topic_id: topic_id.to_string(),
                id: chat_id.clone(),
                seq: msg.seq.unwrap_or(seq),
                created_at: if msg.created_at.is_empty() {
                    Utc::now().to_rfc3339()
                } else {
                    msg.created_at
                },
                sender_id: msg.sender_id,
                content,
                ..ChatLog::default()
            };

            let active: chat_log::ActiveModel = log.into();
            active.insert(&self.db).await?;
            ids.push(chat_id);
        }

        Ok(OpenApiImportTopicMessageResponse { chat_ids: ids })
    }
}
