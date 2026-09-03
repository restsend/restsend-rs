use chrono::Utc;
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    QueryOrder, QuerySelect,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::entity::{chat_log, topic};
use crate::services::topic::{pair_topic_id, peer_pair_topic_id};
use crate::services::{DomainError, DomainResult};
use crate::{
    ChatLog, ChatLogSyncForm, ChatLogSyncResult, OpenApiChatMessageForm,
    OpenApiImportTopicMessageForm, OpenApiImportTopicMessageResponse, OpenApiSendMessageResponse,
};

const SEQ_RETRIES: usize = 8;
const DEFAULT_RECALL_TIMEOUT_SECS: u64 = 120;
const MAX_LOG_LIMIT: u64 = 1000;
/// Idle per-topic seq locks are recycled after this long (memory bound).
const SEQ_LOCK_IDLE_SECS: u64 = 600;
/// Lock-table sweep trigger size.
const SEQ_LOCK_SWEEP_SIZE: usize = 4096;

type SeqLock = Arc<tokio::sync::Mutex<()>>;

#[derive(Clone)]
pub struct ChatService {
    db: DatabaseConnection,
    recall_timeout_secs: Arc<std::sync::atomic::AtomicU64>,
    seq_locks: Arc<Mutex<HashMap<String, (SeqLock, std::time::Instant)>>>,
    relation_service: Option<crate::services::RelationService>,
}

impl ChatService {
    pub fn new(db: DatabaseConnection) -> Self {
        let relation_service = crate::services::RelationService::new(db.clone());
        Self {
            db,
            recall_timeout_secs: Arc::new(std::sync::atomic::AtomicU64::new(
                DEFAULT_RECALL_TIMEOUT_SECS,
            )),
            seq_locks: Arc::new(Mutex::new(HashMap::new())),
            relation_service: Some(relation_service),
        }
    }

    /// Test seam: disable relation checks (used by focused unit tests).
    #[cfg(test)]
    pub fn without_relations(mut self) -> Self {
        self.relation_service = None;
        self
    }

    pub fn set_recall_timeout_secs(&self, secs: u64) {
        self.recall_timeout_secs
            .store(secs, std::sync::atomic::Ordering::Relaxed);
    }

    fn recall_timeout_secs(&self) -> u64 {
        self.recall_timeout_secs
            .load(std::sync::atomic::Ordering::Relaxed)
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

        let (self_topic, peer_topic) = self
            .resolve_dm(sender_id, Some(topic_id.to_string()), None)
            .await?;

        let mut targets = Vec::new();
        for candidate_topic in [Some(&self_topic), peer_topic.as_ref()] {
            let Some(candidate) = candidate_topic else {
                continue;
            };
            if let Some(target) = chat_log::Entity::find()
                .filter(chat_log::Column::TopicId.eq(candidate.to_string()))
                .filter(chat_log::Column::Id.eq(recall_chat_id.to_string()))
                .filter(chat_log::Column::SenderId.eq(sender_id.to_string()))
                .one(&self.db)
                .await?
            {
                targets.push(target);
            }
        }
        let primary = targets
            .first()
            .ok_or_else(|| DomainError::Validation("recall target not found".to_string()))?;

        if primary.recall {
            return Err(DomainError::Validation(
                "recall target already recalled".to_string(),
            ));
        }

        if self.recall_timeout_secs() > 0 {
            let created =
                chrono::DateTime::parse_from_rfc3339(&primary.created_at).ok();
            if let Some(created) = created {
                let elapsed = Utc::now().signed_duration_since(created);
                if elapsed.num_seconds() > self.recall_timeout_secs() as i64 {
                    return Err(DomainError::Validation(
                        "recall timeout exceeded".to_string(),
                    ));
                }
            }
        }

        for target in targets {
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
        }

        self.send_internal(Some(self_topic), sender_id, None, form)
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

        let (self_topic, peer_topic) = self
            .resolve_dm(sender_id, Some(topic_id.to_string()), None)
            .await?;

        for candidate_topic in [Some(&self_topic), peer_topic.as_ref()] {
            let Some(candidate) = candidate_topic else {
                continue;
            };
            let Some(target) = chat_log::Entity::find()
                .filter(chat_log::Column::TopicId.eq(candidate.to_string()))
                .filter(chat_log::Column::Id.eq(target_chat_id.to_string()))
                .filter(chat_log::Column::SenderId.eq(sender_id.to_string()))
                .one(&self.db)
                .await?
            else {
                continue;
            };
            if target.recall {
                return Err(DomainError::Validation(
                    "update extra target already recalled".to_string(),
                ));
            }

            let mut updated_content: crate::Content =
                crate::entity::decode_json(&target.content_json);
            updated_content.extra = content.extra.clone();

            let mut target_active = target.into_active_model();
            target_active.content_json = sea_orm::ActiveValue::Set(
                serde_json::to_string(&updated_content)
                    .map_err(|e| DomainError::Validation(e.to_string()))?,
            );
            target_active.update(&self.db).await?;
        }

        self.send_internal(Some(self_topic), sender_id, None, form)
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

    /// Resolve the sender-centric DM pair topics for a message.
    ///
    /// Returns `(self_topic_id, peer_topic_id)`:
    /// - group chats or self-chats: peer is `None`
    /// - DM: self topic `{sender}:{attendee}`, peer topic `{attendee}:{sender}`
    async fn resolve_dm(
        &self,
        sender_id: &str,
        topic_id: Option<String>,
        attendee_id: Option<String>,
    ) -> DomainResult<(String, Option<String>)> {
        if let Some(attendee) =
            attendee_id.filter(|v| !v.trim().is_empty())
        {
            return self.ensure_pair(sender_id, &attendee).await;
        }

        let topic_id = topic_id
            .filter(|v| !v.trim().is_empty())
            .ok_or_else(|| {
                DomainError::Validation("topicId or attendee is required".to_string())
            })?;
        let topic = match topic::Entity::find_by_id(topic_id.clone())
            .one(&self.db)
            .await?
        {
            Some(topic) => topic,
            None => {
                // A DM send may compute the pair topic id without the pair
                // topics existing yet (e.g. first WS message to an attendee).
                // The chat service owns creating both pair topics on send.
                if let Some((a, b)) = topic_id.split_once(':') {
                    if !a.is_empty() && !b.is_empty() && a != b {
                        let attendee = if a == sender_id {
                            b
                        } else if b == sender_id {
                            a
                        } else {
                            return Err(DomainError::NotFound);
                        };
                        return self.ensure_pair(sender_id, attendee).await;
                    }
                }
                return Err(DomainError::NotFound);
            }
        };
        if topic.multiple {
            return Ok((topic_id, None));
        }

        let attendee = if !topic.attendee_id.is_empty() && topic.attendee_id != sender_id {
            topic.attendee_id.clone()
        } else if !topic.owner_id.is_empty() && topic.owner_id != sender_id {
            topic.owner_id.clone()
        } else {
            return Ok((topic_id, None));
        };
        self.ensure_pair(sender_id, &attendee).await
    }

    async fn ensure_pair(
        &self,
        sender_id: &str,
        attendee_id: &str,
    ) -> DomainResult<(String, Option<String>)> {
        let self_id = pair_topic_id(sender_id, attendee_id);
        let peer_id = peer_pair_topic_id(&self_id);
        self.ensure_pair_topic(&self_id, sender_id, attendee_id)
            .await?;
        if let Some(peer) = peer_id.as_deref() {
            self.ensure_pair_topic(peer, attendee_id, sender_id).await?;
        }
        Ok((self_id, peer_id))
    }

    async fn ensure_pair_topic(
        &self,
        topic_id: &str,
        owner_id: &str,
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
            owner_id: owner_id.to_string(),
            attendee_id: attendee_id.to_string(),
            members: 2,
            multiple: false,
            name: format!("DM with {attendee_id}"),
            source: "pair".to_string(),
            enabled: true,
            created_at: now.clone(),
            updated_at: now,
            ..crate::Topic::default()
        };
        let active: topic::ActiveModel = (row, topic_id).into();
        let _ = active.insert(&self.db).await?;
        Ok(())
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

        let (self_topic, peer_topic) = self
            .resolve_dm(sender_id, topic_id, attendee_id.clone())
            .await?;
        self.ensure_topic_enabled(&self_topic).await?;

        let base_content = form.content.clone().unwrap_or_else(|| crate::Content {
            content_type: if form.r#type.is_empty() {
                "chat".to_string()
            } else {
                form.r#type.clone()
            },
            text: form.message.clone(),
            ..crate::Content::default()
        });

        let self_seq = self.next_topic_seq(&self_topic).await?;
        self.insert_log(&self_topic, &chat_id, self_seq, sender_id, &base_content, form)
            .await?;

        if let Some(peer_topic) = peer_topic.as_deref() {
            self.ensure_topic_enabled(peer_topic).await?;
            let peer_content = peer_view_content(&base_content, form);
            let peer_seq = self.next_topic_seq(peer_topic).await?;
            self.insert_log(
                peer_topic,
                &chat_id,
                peer_seq,
                sender_id,
                &peer_content,
                form,
            )
            .await?;
        }

        Ok(OpenApiSendMessageResponse {
            sender_id: sender_id.to_string(),
            topic_id: self_topic,
            attendee_id: attendee_id.unwrap_or_default(),
            chat_id,
            code: 200,
            message: "ok".to_string(),
            seq: self_seq,
            usage: 0,
        })
    }

    async fn insert_log(
        &self,
        topic_id: &str,
        chat_id: &str,
        seq: i64,
        sender_id: &str,
        content: &crate::Content,
        form: &OpenApiChatMessageForm,
    ) -> DomainResult<()> {
        let mut content = content.clone();
        // Backfill replyContent from the replied-to message (skip recalled).
        if !content.reply.is_empty() {
            if let Some(target) = chat_log::Entity::find()
                .filter(chat_log::Column::TopicId.eq(topic_id.to_string()))
                .filter(chat_log::Column::Id.eq(content.reply.clone()))
                .one(&self.db)
                .await?
            {
                if !target.recall {
                    content.reply_content = Some(target.content_json);
                }
            }
        }

        let log = ChatLog {
            topic_id: topic_id.to_string(),
            id: chat_id.to_string(),
            seq,
            created_at: if let Some(ts) = &form.created_at {
                ts.clone()
            } else {
                Utc::now().to_rfc3339()
            },
            sender_id: sender_id.to_string(),
            content,
            ..ChatLog::default()
        };

        let active: chat_log::ActiveModel = log.into();
        active.insert(&self.db).await?;
        Ok(())
    }

    async fn next_topic_seq(&self, topic_id: &str) -> DomainResult<i64> {
        let lock = {
            let mut map = self
                .seq_locks
                .lock()
                .map_err(|_| DomainError::Storage("seq lock poisoned".to_string()))?;
            // Recycle idle locks so the table stays bounded on servers with
            // a long-tail of topics (entry is dropped once no sender holds
            // it and it has not been used for SEQ_LOCK_IDLE_SECS).
            if map.len() >= SEQ_LOCK_SWEEP_SIZE {
                sweep_idle_locks(&mut map);
            }
            let entry = map
                .entry(topic_id.to_string())
                .and_modify(|(_, last_used)| *last_used = std::time::Instant::now())
                .or_insert_with(|| {
                    (
                        Arc::new(tokio::sync::Mutex::new(())),
                        std::time::Instant::now(),
                    )
                });
            entry.1 = std::time::Instant::now();
            entry.0.clone()
        };
        let _guard = lock.lock().await;

        for _ in 0..SEQ_RETRIES {
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
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        Err(DomainError::Conflict)
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

        let limit = form.limit.unwrap_or(50).clamp(1, MAX_LOG_LIMIT);
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

/// Peer-side view of a message: when the sender supplied `e2eContent`, the peer
/// copy stores it in place of the plaintext `text` (Go dual-topic E2E model).
fn peer_view_content(content: &crate::Content, form: &OpenApiChatMessageForm) -> crate::Content {
    if form.e2e_content.trim().is_empty() {
        return content.clone();
    }
    let mut peer = content.clone();
    peer.text = form.e2e_content.clone();
    peer
}

fn sweep_idle_locks(
    map: &mut HashMap<String, (SeqLock, std::time::Instant)>,
) {
    let now = std::time::Instant::now();
    map.retain(|_, (lock, last_used)| {
        now.duration_since(*last_used).as_secs() < SEQ_LOCK_IDLE_SECS || lock.try_lock().is_err()
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seq_lock_sweep_recycles_idle_entries_only() {
        let mut map: HashMap<String, (SeqLock, std::time::Instant)> = HashMap::new();
        let stale = std::time::Instant::now() - std::time::Duration::from_secs(SEQ_LOCK_IDLE_SECS + 1);
        let fresh = std::time::Instant::now();
        // stale idle -> dropped
        map.insert("topic-stale".into(), (Arc::new(tokio::sync::Mutex::new(())), stale));
        // fresh -> kept
        map.insert("topic-fresh".into(), (Arc::new(tokio::sync::Mutex::new(())), fresh));
        // stale but actively locked -> kept (leak the guard so the mutex
        // stays locked for the duration of the test)
        let busy = Arc::new(tokio::sync::Mutex::new(()));
        std::mem::forget(busy.try_lock());
        map.insert("topic-busy".into(), (busy, stale));

        sweep_idle_locks(&mut map);
        assert!(map.contains_key("topic-fresh"));
        assert!(map.contains_key("topic-busy"));
        assert!(!map.contains_key("topic-stale"));
    }

    #[test]
    fn peer_view_replaces_text_with_e2e() {
        let form = OpenApiChatMessageForm {
            r#type: "chat".to_string(),
            message: "hello".to_string(),
            e2e_content: "ciphertext".to_string(),
            ..Default::default()
        };
        let content = crate::Content {
            content_type: "chat".to_string(),
            text: "hello".to_string(),
            ..Default::default()
        };
        let peer = peer_view_content(&content, &form);
        assert_eq!(peer.text, "ciphertext");
        assert_eq!(content.text, "hello");
    }

    #[test]
    fn peer_view_keeps_content_without_e2e() {
        let form = OpenApiChatMessageForm::default();
        let content = crate::Content {
            content_type: "chat".to_string(),
            text: "hello".to_string(),
            ..Default::default()
        };
        let peer = peer_view_content(&content, &form);
        assert_eq!(peer.text, "hello");
    }
}
