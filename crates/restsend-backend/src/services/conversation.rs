use chrono::Utc;
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    IntoActiveModel, QueryFilter, QueryOrder, QuerySelect,
};
use std::collections::HashMap;

use crate::entity::conversation;
use crate::entity::{decode_json, encode_json};
use crate::services::{DomainError, DomainResult};
use crate::{Conversation, OpenApiUpdateConversationForm};

/// Parameters for [`ConversationService::advance_last_message`].
///
/// `bump_unread` should be false for the sender's own row.
/// `draft` supplies the initial field values if the row has to be created.
#[derive(Debug, Clone)]
pub struct AdvanceLastMessage {
    pub seq: i64,
    pub sender_id: String,
    pub content: Option<crate::Content>,
    pub last_message_at: String,
    pub bump_unread: bool,
    pub draft: Conversation,
}

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
            active.deleted_at = Set(None);
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
        let current_extra_json = existing.extra_json.clone();

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
        if let Some(_tags) = form.tags {
            active.tags_json = Set(encode_json(&_tags));
        }
        if let Some(new_extra) = form.extra {
            // Merge per-key so multiple features sharing conversation.extra
            // (e.g. "replied", "draft") do not clobber each other.
            // An empty map keeps the historical "clear all" behavior.
            if new_extra.is_empty() {
                active.extra_json = Set(encode_json(&new_extra));
            } else {
                let mut merged: std::collections::HashMap<String, String> =
                    decode_json(&current_extra_json);
                for (k, v) in new_extra {
                    merged.insert(k, v);
                }
                active.extra_json = Set(encode_json(&merged));
            }
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
        // reading restores a soft-deleted conversation
        active.deleted_at = Set(None);
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
            active.deleted_at = Set(None);
            active.updated_at = Set(now());
            let _ = active.update(&self.db).await?;
            changed += 1;
        }

        Ok(changed)
    }

    /// Category filter: "" / "all" = none; sticky/mute/unread are conversation
    /// flags; personal/group/vip/system map to topic kind & multiple.
    fn apply_category_filter(
        query: sea_orm::Select<conversation::Entity>,
        category: &str,
    ) -> sea_orm::Select<conversation::Entity> {
        match category.trim().to_ascii_lowercase().as_str() {
            "sticky" => query.filter(conversation::Column::Sticky.eq(true)),
            "mute" => query.filter(conversation::Column::Mute.eq(true)),
            "unread" => query.filter(conversation::Column::Unread.gt(0)),
            "personal" => query.filter(conversation::Column::Multiple.eq(false)),
            "group" => query.filter(conversation::Column::Multiple.eq(true)),
            "vip" | "system" => query.filter(conversation::Column::Kind.eq(category.trim())),
            _ => query,
        }
    }

    pub async fn list_by_user(
        &self,
        owner_id: &str,
        offset: u64,
        limit: u64,
    ) -> DomainResult<Vec<Conversation>> {
        self.list_by_user_filtered(owner_id, offset, limit, "").await
    }

    pub async fn list_by_user_filtered(
        &self,
        owner_id: &str,
        offset: u64,
        limit: u64,
        category: &str,
    ) -> DomainResult<Vec<Conversation>> {
        let st = std::time::Instant::now();
        let query = conversation::Entity::find()
            .filter(conversation::Column::OwnerId.eq(owner_id.to_string()))
            .filter(conversation::Column::DeletedAt.is_null());
        let query = Self::apply_category_filter(query, category);
        let rows: Vec<conversation::Model> = query
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
            category = %category,
            count = result.len(),
            elapsed_ms = st.elapsed().as_millis() as u64,
            "conversation list by user"
        );
        Ok(result)
    }

    /// Incremental sync: conversations updated at or after `updated_at`,
    /// ordered ascending, `limit + 1` probe for hasMore (Go sync semantics).
    /// Pass `category` to filter; `until` bounds the cursor for paging
    /// (`lastUpdatedAt <= until`).
    pub async fn list_updated_since(
        &self,
        owner_id: &str,
        updated_at: &str,
        until: Option<&str>,
        limit: u64,
        category: &str,
    ) -> DomainResult<(Vec<Conversation>, bool)> {
        let query = conversation::Entity::find()
            .filter(conversation::Column::OwnerId.eq(owner_id.to_string()))
            .filter(conversation::Column::DeletedAt.is_null())
            .filter(conversation::Column::UpdatedAt.gte(updated_at.to_string()));
        let query = match until {
            Some(until) if !until.is_empty() => {
                query.filter(conversation::Column::UpdatedAt.lte(until.to_string()))
            }
            _ => query,
        };
        let query = Self::apply_category_filter(query, category);
        let rows: Vec<conversation::Model> = query
            .order_by_asc(conversation::Column::UpdatedAt)
            .limit(limit + 1)
            .all(&self.db)
            .await?;
        let has_more = rows.len() as u64 > limit;
        let items = rows
            .into_iter()
            .take(limit as usize)
            .map(Conversation::from)
            .collect();
        Ok((items, has_more))
    }

    /// Topic ids of soft-deleted conversations (removed sync list).
    pub async fn list_removed(
        &self,
        owner_id: &str,
        since: Option<&str>,
        limit: u64,
    ) -> DomainResult<Vec<String>> {
        let mut query = conversation::Entity::find()
            .filter(conversation::Column::OwnerId.eq(owner_id.to_string()))
            .filter(conversation::Column::DeletedAt.is_not_null());
        if let Some(since) = since.filter(|v| !v.is_empty()) {
            query = query.filter(conversation::Column::DeletedAt.gte(since.to_string()));
        }
        let rows: Vec<conversation::Model> = query
            .order_by_asc(conversation::Column::DeletedAt)
            .limit(limit.clamp(1, 1000))
            .all(&self.db)
            .await?;
        Ok(rows.into_iter().map(|v| v.topic_id).collect())
    }

    /// Per-category totals/unread for the conversation list home request.
    pub async fn build_categories(
        &self,
        owner_id: &str,
    ) -> DomainResult<Vec<crate::CategorySummary>> {
        let rows: Vec<conversation::Model> = conversation::Entity::find()
            .filter(conversation::Column::OwnerId.eq(owner_id.to_string()))
            .filter(conversation::Column::DeletedAt.is_null())
            .all(&self.db)
            .await?;

        let mut all_total = 0i64;
        let mut all_unread = 0i64;
        let mut sticky = (0i64, 0i64);
        let mut mute = (0i64, 0i64);
        let mut unread = (0i64, 0i64);
        let mut personal = (0i64, 0i64);
        let mut group = (0i64, 0i64);
        let mut kinds: HashMap<String, (i64, i64)> = HashMap::new();

        for row in &rows {
            let total = &mut all_total;
            *total += 1;
            all_unread += row.unread;
            if row.sticky {
                sticky.0 += 1;
                sticky.1 += row.unread;
            }
            if row.mute {
                mute.0 += 1;
                mute.1 += row.unread;
            }
            if row.unread > 0 {
                unread.0 += 1;
                unread.1 += row.unread;
            }
            if row.multiple {
                group.0 += 1;
                group.1 += row.unread;
            } else {
                personal.0 += 1;
                personal.1 += row.unread;
            }
            if !row.kind.is_empty() {
                let entry = kinds.entry(row.kind.clone()).or_insert((0, 0));
                entry.0 += 1;
                entry.1 += row.unread;
            }
        }

        let mut out = vec![
            crate::CategorySummary { name: "all".into(), total: all_total, unread: all_unread },
            crate::CategorySummary { name: "sticky".into(), total: sticky.0, unread: sticky.1 },
            crate::CategorySummary { name: "mute".into(), total: mute.0, unread: mute.1 },
            crate::CategorySummary { name: "unread".into(), total: unread.0, unread: unread.1 },
            crate::CategorySummary { name: "personal".into(), total: personal.0, unread: personal.1 },
            crate::CategorySummary { name: "group".into(), total: group.0, unread: group.1 },
        ];
        for (kind, (total, unread)) in kinds {
            if kind == "vip" || kind == "system" {
                out.push(crate::CategorySummary { name: kind, total, unread });
            }
        }
        Ok(out)
    }

    pub async fn remove_conversation(&self, owner_id: &str, topic_id: &str) -> DomainResult<()> {
        let existing =
            conversation::Entity::find_by_id((owner_id.to_string(), topic_id.to_string()))
                .one(&self.db)
                .await?
                .ok_or(DomainError::NotFound)?;
        // Soft delete so other devices can sync the removal.
        if existing.deleted_at.is_some() {
            return Ok(());
        }
        let mut active = existing.into_active_model();
        active.deleted_at = Set(Some(now()));
        active.updated_at = Set(now());
        active.update(&self.db).await?;
        Ok(())
    }

    pub async fn update_last_seq(
        &self,
        owner_id: &str,
        topic_id: &str,
        seq: i64,
    ) -> DomainResult<()> {
        // Guarded advance: a late writer carrying an older seq must never move
        // last_seq backwards.
        conversation::Entity::update_many()
            .col_expr(conversation::Column::LastSeq, seq.into())
            .col_expr(conversation::Column::UpdatedAt, now().into())
            .filter(conversation::Column::OwnerId.eq(owner_id.to_string()))
            .filter(conversation::Column::TopicId.eq(topic_id.to_string()))
            .filter(conversation::Column::LastSeq.lt(seq))
            .exec(&self.db)
            .await?;
        Ok(())
    }

    /// Advance the denormalized last-message snapshot of one member's
    /// conversation as part of delivering a new chat message.
    ///
    /// Concurrency contract (this is what keeps the conversation list
    /// consistent with the message panel under concurrent sends):
    /// - `last_message_*`/`last_seq` only ever move forward: each UPDATE is
    ///   guarded by `last_message_seq < seq` / `last_seq < seq`, so a delayed
    ///   writer holding an older message can never regress the row.
    /// - `unread` is incremented with a single atomic
    ///   `unread = unread + 1` statement (no read-modify-write), so concurrent
    ///   messages never lose counts.
    /// - User settings (sticky/mute/remark/start_seq/last_read_seq/...) are
    ///   never touched here; they are only applied by `create_or_update` /
    ///   `update_conversation`.
    pub async fn advance_last_message(
        &self,
        owner_id: &str,
        topic_id: &str,
        msg: AdvanceLastMessage,
    ) -> DomainResult<Conversation> {
        let AdvanceLastMessage {
            seq,
            sender_id,
            content,
            last_message_at,
            bump_unread,
            draft,
        } = msg;
        let content_json = content
            .as_ref()
            .map(encode_json)
            .unwrap_or_else(|| "{}".to_string());

        for _ in 0..3 {
            if bump_unread {
                conversation::Entity::update_many()
                    .col_expr(
                        conversation::Column::Unread,
                        Expr::col(conversation::Column::Unread).add(1),
                    )
                    .filter(conversation::Column::OwnerId.eq(owner_id.to_string()))
                    .filter(conversation::Column::TopicId.eq(topic_id.to_string()))
                    .exec(&self.db)
                    .await?;
            }

            conversation::Entity::update_many()
                .col_expr(
                    conversation::Column::LastMessageSeq,
                    seq.into(),
                )
                .col_expr(
                    conversation::Column::LastSenderId,
                    sender_id.to_string().into(),
                )
                .col_expr(
                    conversation::Column::LastMessageJson,
                    content_json.clone().into(),
                )
                .col_expr(
                    conversation::Column::LastMessageAt,
                    last_message_at.clone().into(),
                )
                .col_expr(conversation::Column::UpdatedAt, now().into())
                .filter(conversation::Column::OwnerId.eq(owner_id.to_string()))
                .filter(conversation::Column::TopicId.eq(topic_id.to_string()))
                .filter(conversation::Column::LastMessageSeq.lt(seq))
                .exec(&self.db)
                .await?;

            conversation::Entity::update_many()
                .col_expr(conversation::Column::LastSeq, seq.into())
                .filter(conversation::Column::OwnerId.eq(owner_id.to_string()))
                .filter(conversation::Column::TopicId.eq(topic_id.to_string()))
                .filter(conversation::Column::LastSeq.lt(seq))
                .exec(&self.db)
                .await?;

            if let Some(existing) = conversation::Entity::find_by_id((
                owner_id.to_string(),
                topic_id.to_string(),
            ))
            .one(&self.db)
            .await?
            {
                return Ok(existing.into());
            }

            // Row does not exist yet: create it once. On a primary-key race the
            // insert is skipped and the loop retries, so the increments above
            // still land exactly once.
            let created: conversation::ActiveModel = (
                Conversation {
                    owner_id: owner_id.to_string(),
                    topic_id: topic_id.to_string(),
                    unread: if bump_unread { 1 } else { 0 },
                    last_seq: seq,
                    last_sender_id: sender_id.to_string(),
                    last_message: content.clone(),
                    last_message_at: last_message_at.clone(),
                    last_message_seq: Some(seq),
                    updated_at: now(),
                    ..draft.clone()
                },
                now().as_str(),
            )
                .into();
            match conversation::Entity::insert(created)
                .on_conflict(
                    sea_orm::sea_query::OnConflict::new()
                        .do_nothing()
                        .to_owned(),
                )
                .exec(&self.db)
                .await
            {
                Ok(_) => {
                    if let Some(existing) = conversation::Entity::find_by_id((
                        owner_id.to_string(),
                        topic_id.to_string(),
                    ))
                    .one(&self.db)
                    .await?
                    {
                        return Ok(existing.into());
                    }
                }
                Err(sea_orm::DbErr::RecordNotInserted) => continue,
                Err(e) => return Err(e.into()),
            }
        }

        Err(DomainError::Storage(
            "conversation advance retry exhausted".to_string(),
        ))
    }
}

fn now() -> String {
    Utc::now().to_rfc3339()
}
