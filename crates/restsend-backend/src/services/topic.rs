use chrono::{Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    IntoActiveModel, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
};

use crate::entity::{topic, topic_knock, topic_member};
use crate::services::{DomainError, DomainResult};
use crate::{
    ListUserResult, OpenApiCreateTopicForm, OpenApiSilentTopicForm, OpenApiSilentTopicMembersForm,
    OpenApiUpdateTopicExtraForm, OpenApiUpdateTopicForm, OpenApiUpdateTopicMemberForm, Topic,
    TopicKnock, TopicKnockAcceptedForm, TopicKnockForm, TopicKnockRejectedForm, TopicMember,
    UpdateNoticeForm,
};

#[derive(Clone)]
pub struct TopicService {
    db: DatabaseConnection,
}

impl TopicService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn get_by_id(&self, topic_id: &str) -> DomainResult<Topic> {
        let model = topic::Entity::find_by_id(topic_id.to_string())
            .one(&self.db)
            .await?
            .ok_or(DomainError::NotFound)?;
        if !model.enabled {
            return Err(DomainError::Forbidden);
        }
        Ok(model.into())
    }

    pub async fn get_any_by_id(&self, topic_id: &str) -> DomainResult<Topic> {
        let model = topic::Entity::find_by_id(topic_id.to_string())
            .one(&self.db)
            .await?
            .ok_or(DomainError::NotFound)?;
        Ok(model.into())
    }

    pub async fn create_topic(
        &self,
        topic_id: Option<String>,
        form: OpenApiCreateTopicForm,
    ) -> DomainResult<Topic> {
        let id = topic_id.unwrap_or_else(|| format!("topic-{}", uuid::Uuid::new_v4().simple()));
        let owner_id = if form.without_owner {
            String::new()
        } else if form.sender_id.is_empty() {
            if form.members.is_empty() {
                String::new()
            } else {
                form.members[0].clone()
            }
        } else {
            form.sender_id.clone()
        };

        let multiple = form.multiple.unwrap_or(true);
        let filtered_members: Vec<String> = if multiple {
            let mut members = Vec::new();
            for user_id in form.members {
                if user_id.trim().is_empty() || members.iter().any(|v| v == &user_id) {
                    continue;
                }
                if crate::entity::user::Entity::find_by_id(user_id.clone())
                    .one(&self.db)
                    .await?
                    .is_some()
                {
                    members.push(user_id);
                }
            }
            if !owner_id.is_empty() && !members.iter().any(|v| v == &owner_id) {
                members.insert(0, owner_id.clone());
            }
            members
        } else {
            let mut members = Vec::new();
            if !owner_id.is_empty() {
                members.push(owner_id.clone());
            }
            for user_id in form.members {
                if user_id.trim().is_empty() || members.iter().any(|v| v == &user_id) {
                    continue;
                }
                if crate::entity::user::Entity::find_by_id(user_id.clone())
                    .one(&self.db)
                    .await?
                    .is_some()
                {
                    members.push(user_id);
                    break;
                }
            }
            members
        };

        if let Some(can_override) = form.can_override {
            if !can_override
                && topic::Entity::find_by_id(id.clone())
                    .one(&self.db)
                    .await?
                    .is_some()
            {
                return Err(DomainError::Conflict);
            }
        }

        let now = now();
        let topic = Topic {
            id: id.clone(),
            name: form.name,
            icon: form.icon,
            kind: form.kind,
            owner_id,
            members: filtered_members.len() as u32,
            multiple,
            source: form.source,
            private: form.private.unwrap_or(false),
            knock_need_verify: form.knock_need_verify.unwrap_or(false),
            admins: form.admins,
            webhooks: form.webhooks,
            notice: form.notice.map(|v| crate::TopicNotice {
                text: v.text,
                publisher: v.publisher,
                updated_at: v.updated_at,
            }),
            extra: form.extra,
            enabled: true,
            created_at: now.clone(),
            updated_at: now.clone(),
            ..Topic::default()
        };

        let active: topic::ActiveModel = (topic, now.as_str()).into();
        let created = active.insert(&self.db).await?;

        if form.ensure_conversation.unwrap_or(false) || !filtered_members.is_empty() {
            for user_id in filtered_members {
                let member = TopicMember {
                    topic_id: created.id.clone(),
                    user_id,
                    source: "openapi".to_string(),
                    joined_at: now.clone(),
                    ..TopicMember::default()
                };
                let active: topic_member::ActiveModel = (member, now.as_str()).into();
                let _ = active.insert(&self.db).await;
            }
        }

        Ok(created.into())
    }

    pub async fn update_topic(
        &self,
        topic_id: &str,
        form: OpenApiUpdateTopicForm,
    ) -> DomainResult<Topic> {
        let existing = topic::Entity::find_by_id(topic_id.to_string())
            .one(&self.db)
            .await?
            .ok_or(DomainError::NotFound)?;

        let mut active = existing.into_active_model();
        if !form.source.is_empty() {
            active.source = Set(form.source);
        }
        if !form.kind.is_empty() {
            active.kind = Set(form.kind);
        }
        if !form.name.is_empty() {
            active.name = Set(form.name);
        }
        if !form.icon.is_empty() {
            active.icon = Set(form.icon);
        }
        if !form.admins.is_empty() {
            active.admins_json =
                Set(serde_json::to_string(&form.admins).unwrap_or_else(|_| "[]".to_string()));
        }
        if let Some(v) = form.private {
            active.private = Set(v);
        }
        if let Some(v) = form.knock_need_verify {
            active.knock_need_verify = Set(v);
        }
        if !form.webhooks.is_empty() {
            active.webhooks_json =
                Set(serde_json::to_string(&form.webhooks).unwrap_or_else(|_| "[]".to_string()));
        }
        if let Some(v) = form.notice {
            active.notice_json = Set(serde_json::to_string(&crate::TopicNotice {
                text: v.text,
                publisher: v.publisher,
                updated_at: v.updated_at,
            })
            .unwrap_or_else(|_| "{}".to_string()));
        }
        if let Some(v) = form.extra {
            active.extra_json = Set(serde_json::to_string(&v).unwrap_or_else(|_| "{}".to_string()));
        }
        active.updated_at = Set(now());

        let updated = active.update(&self.db).await?;
        Ok(updated.into())
    }

    pub async fn update_topic_extra(
        &self,
        topic_id: &str,
        form: OpenApiUpdateTopicExtraForm,
    ) -> DomainResult<Topic> {
        let existing = topic::Entity::find_by_id(topic_id.to_string())
            .one(&self.db)
            .await?
            .ok_or(DomainError::NotFound)?;

        let mut current_extra: std::collections::HashMap<String, String> =
            serde_json::from_str(&existing.extra_json).unwrap_or_default();
        for action in form.actions {
            match action.action.as_str() {
                "remove" => {
                    current_extra.remove(&action.key);
                }
                "set" | "upsert" | "add" => {
                    if let Some(v) = action.value {
                        current_extra.insert(action.key, v);
                    }
                }
                _ => {}
            }
        }

        let mut active = existing.into_active_model();
        if !form.source.is_empty() {
            active.source = Set(form.source);
        }
        active.extra_json =
            Set(serde_json::to_string(&current_extra).unwrap_or_else(|_| "{}".to_string()));
        active.updated_at = Set(now());

        let updated = active.update(&self.db).await?;
        Ok(updated.into())
    }

    pub async fn list_members(&self, topic_id: &str) -> DomainResult<Vec<String>> {
        let rows = topic_member::Entity::find()
            .filter(topic_member::Column::TopicId.eq(topic_id.to_string()))
            .all(&self.db)
            .await?;
        Ok(rows.into_iter().map(|v| v.user_id).collect())
    }

    pub async fn list_members_detailed(
        &self,
        topic_id: &str,
        _updated_at: Option<&str>,
        limit: Option<u64>,
    ) -> DomainResult<ListUserResult> {
        let topic_rows = topic_member::Entity::find()
            .filter(topic_member::Column::TopicId.eq(topic_id.to_string()))
            .all(&self.db)
            .await?;

        let max = limit.unwrap_or(100).clamp(1, 500) as usize;
        let mut items = Vec::new();
        for row in topic_rows.into_iter().take(max) {
            if let Ok(user) = crate::services::UserService::new(self.db.clone())
                .get_by_user_id(&row.user_id)
                .await
            {
                items.push(user);
            }
        }

        Ok(ListUserResult {
            has_more: false,
            updated_at: now(),
            items,
        })
    }

    pub async fn join_members(
        &self,
        topic_id: &str,
        user_ids: Vec<String>,
        source: String,
    ) -> DomainResult<Vec<String>> {
        let mut out = Vec::with_capacity(user_ids.len());
        let now = now();
        for user_id in user_ids {
            let existing =
                topic_member::Entity::find_by_id((topic_id.to_string(), user_id.clone()))
                    .one(&self.db)
                    .await?;
            if existing.is_none() {
                let member = TopicMember {
                    topic_id: topic_id.to_string(),
                    user_id: user_id.clone(),
                    source: source.clone(),
                    joined_at: now.clone(),
                    ..TopicMember::default()
                };
                let active: topic_member::ActiveModel = (member, now.as_str()).into();
                let _ = active.insert(&self.db).await?;
            }
            out.push(user_id);
        }
        self.refresh_members_count(topic_id).await?;
        Ok(out)
    }

    pub async fn quit_members(
        &self,
        topic_id: &str,
        user_ids: Vec<String>,
    ) -> DomainResult<Vec<String>> {
        let mut out = Vec::with_capacity(user_ids.len());
        for user_id in user_ids {
            let rows = topic_member::Entity::delete_by_id((topic_id.to_string(), user_id.clone()))
                .exec(&self.db)
                .await?
                .rows_affected;
            if rows > 0 {
                out.push(user_id);
            }
        }
        self.refresh_members_count(topic_id).await?;
        Ok(out)
    }

    pub async fn dismiss_topic(&self, topic_id: &str) -> DomainResult<()> {
        let rows = topic::Entity::delete_by_id(topic_id.to_string())
            .exec(&self.db)
            .await?
            .rows_affected;
        let _ = topic_member::Entity::delete_many()
            .filter(topic_member::Column::TopicId.eq(topic_id.to_string()))
            .exec(&self.db)
            .await;
        if rows == 0 {
            return Err(DomainError::NotFound);
        }
        Ok(())
    }

    pub async fn set_enabled(&self, topic_id: &str, enabled: bool) -> DomainResult<Topic> {
        let existing = topic::Entity::find_by_id(topic_id.to_string())
            .one(&self.db)
            .await?
            .ok_or(DomainError::NotFound)?;
        let mut active = existing.into_active_model();
        active.enabled = Set(enabled);
        active.updated_at = Set(now());
        let updated = active.update(&self.db).await?;
        Ok(updated.into())
    }

    pub async fn list_topics(
        &self,
        offset: u64,
        limit: u64,
        keyword: Option<&str>,
    ) -> DomainResult<(Vec<Topic>, u64)> {
        let limit = limit.clamp(1, 200);
        let mut query = topic::Entity::find().order_by_asc(topic::Column::Id);
        if let Some(keyword) = keyword.map(str::trim).filter(|v| !v.is_empty()) {
            query = query.filter(
                topic::Column::Id
                    .contains(keyword)
                    .or(topic::Column::Name.contains(keyword))
                    .or(topic::Column::OwnerId.contains(keyword)),
            );
        }
        let total = query.clone().count(&self.db).await?;
        let rows: Vec<topic::Model> = query.offset(offset).limit(limit).all(&self.db).await?;
        Ok((rows.into_iter().map(Into::into).collect(), total))
    }

    pub async fn update_member(
        &self,
        topic_id: &str,
        user_id: &str,
        form: OpenApiUpdateTopicMemberForm,
    ) -> DomainResult<TopicMember> {
        let existing =
            topic_member::Entity::find_by_id((topic_id.to_string(), user_id.to_string()))
                .one(&self.db)
                .await?
                .ok_or(DomainError::NotFound)?;

        let mut active = existing.into_active_model();
        if let Some(name) = form.name {
            active.name = Set(name);
        }
        if let Some(source) = form.source {
            active.source = Set(source);
        }
        if let Some(extra) = form.extra {
            active.extra_json =
                Set(serde_json::to_string(&extra).unwrap_or_else(|_| "{}".to_string()));
        }
        active.updated_at = Set(now());

        let updated = active.update(&self.db).await?;
        Ok(updated.into())
    }

    pub async fn get_member(&self, topic_id: &str, user_id: &str) -> DomainResult<TopicMember> {
        let row = topic_member::Entity::find_by_id((topic_id.to_string(), user_id.to_string()))
            .one(&self.db)
            .await?
            .ok_or(DomainError::NotFound)?;
        Ok(row.into())
    }

    pub async fn transfer_owner(&self, topic_id: &str, user_id: &str) -> DomainResult<()> {
        let existing = topic::Entity::find_by_id(topic_id.to_string())
            .one(&self.db)
            .await?
            .ok_or(DomainError::NotFound)?;
        let mut admins: Vec<String> =
            serde_json::from_str(&existing.admins_json).unwrap_or_default();
        admins.retain(|admin_id| admin_id != user_id);
        let mut active = existing.into_active_model();
        active.owner_id = Set(user_id.to_string());
        active.admins_json =
            Set(serde_json::to_string(&admins).unwrap_or_else(|_| "[]".to_string()));
        active.updated_at = Set(now());
        let _ = active.update(&self.db).await?;
        Ok(())
    }

    pub async fn add_admin(&self, topic_id: &str, user_id: &str) -> DomainResult<()> {
        let existing = topic::Entity::find_by_id(topic_id.to_string())
            .one(&self.db)
            .await?
            .ok_or(DomainError::NotFound)?;
        if existing.owner_id == user_id {
            return Err(DomainError::Validation(
                "topic owner can't be admin".to_string(),
            ));
        }
        self.modify_admin_list(topic_id, |admins| {
            if !admins.iter().any(|v| v == user_id) {
                admins.push(user_id.to_string());
            }
        })
        .await
    }

    pub async fn remove_admin(&self, topic_id: &str, user_id: &str) -> DomainResult<()> {
        self.modify_admin_list(topic_id, |admins| {
            admins.retain(|v| v != user_id);
        })
        .await
    }

    pub async fn silent_member(
        &self,
        topic_id: &str,
        form: OpenApiSilentTopicMembersForm,
    ) -> DomainResult<Vec<String>> {
        let until = parse_duration_to_time(&form.duration);
        let mut changed = Vec::new();
        for user_id in form.user_ids {
            if let Some(row) =
                topic_member::Entity::find_by_id((topic_id.to_string(), user_id.clone()))
                    .one(&self.db)
                    .await?
            {
                let mut active = row.into_active_model();
                active.silence_at = Set(until.clone());
                active.updated_at = Set(now());
                let _ = active.update(&self.db).await?;
                changed.push(user_id);
            }
        }
        Ok(changed)
    }

    pub async fn add_silent_whitelist(
        &self,
        topic_id: &str,
        user_ids: Vec<String>,
    ) -> DomainResult<Vec<String>> {
        self.modify_silent_whitelist(topic_id, |list| {
            for user_id in &user_ids {
                if !list.iter().any(|v| v == user_id) {
                    list.push(user_id.clone());
                }
            }
        })
        .await
    }

    pub async fn remove_silent_whitelist(
        &self,
        topic_id: &str,
        user_ids: Vec<String>,
    ) -> DomainResult<Vec<String>> {
        self.modify_silent_whitelist(topic_id, |list| {
            list.retain(|v| !user_ids.iter().any(|id| id == v));
        })
        .await
    }

    pub async fn silent_topic(
        &self,
        topic_id: &str,
        form: OpenApiSilentTopicForm,
    ) -> DomainResult<()> {
        let existing = topic::Entity::find_by_id(topic_id.to_string())
            .one(&self.db)
            .await?
            .ok_or(DomainError::NotFound)?;
        let mut active = existing.into_active_model();
        active.silent = Set(!form.duration.is_empty());
        active.updated_at = Set(now());
        let _ = active.update(&self.db).await?;
        Ok(())
    }

    pub async fn add_knock(
        &self,
        topic_id: &str,
        user_id: &str,
        form: TopicKnockForm,
    ) -> DomainResult<()> {
        let topic = self.get_by_id(topic_id).await?;
        if !topic.multiple {
            return Err(DomainError::Validation(
                "not multiple user topic".to_string(),
            ));
        }
        if topic.private {
            return Err(DomainError::Forbidden);
        }

        let exists = topic_member::Entity::find_by_id((topic_id.to_string(), user_id.to_string()))
            .one(&self.db)
            .await?;
        if exists.is_some() {
            return Ok(());
        }

        if !topic.knock_need_verify {
            self.join_members(topic_id, vec![user_id.to_string()], form.source)
                .await?;
            return Ok(());
        }

        let now_ts = now();
        let knock_message = form.message.clone();
        let knock_source = form.source.clone();
        let knock = topic_knock::ActiveModel {
            topic_id: Set(topic_id.to_string()),
            user_id: Set(user_id.to_string()),
            created_at: Set(now_ts.clone()),
            updated_at: Set(now_ts),
            message: Set(knock_message.clone()),
            source: Set(knock_source.clone()),
            status: Set("pending".to_string()),
            admin_id: Set(String::new()),
        };
        match knock.insert(&self.db).await {
            Ok(_) => {}
            Err(_) => {
                let existing =
                    topic_knock::Entity::find_by_id((topic_id.to_string(), user_id.to_string()))
                        .one(&self.db)
                        .await?
                        .ok_or(DomainError::NotFound)?;
                let mut active = existing.into_active_model();
                active.updated_at = Set(now());
                active.message = Set(knock_message);
                active.source = Set(knock_source);
                active.status = Set("pending".to_string());
                active.admin_id = Set(String::new());
                let _ = active.update(&self.db).await?;
            }
        }
        Ok(())
    }

    pub async fn list_pending_knocks(&self, topic_id: &str) -> DomainResult<Vec<TopicKnock>> {
        let rows = topic_knock::Entity::find()
            .filter(topic_knock::Column::TopicId.eq(topic_id.to_string()))
            .filter(topic_knock::Column::Status.eq("pending"))
            .all(&self.db)
            .await?;
        Ok(rows
            .into_iter()
            .map(|row| TopicKnock {
                created_at: row.created_at,
                updated_at: row.updated_at,
                topic_id: row.topic_id,
                user_id: row.user_id,
                message: row.message,
                source: row.source,
                status: row.status,
                admin_id: row.admin_id,
            })
            .collect())
    }

    pub async fn accept_knock(
        &self,
        topic_id: &str,
        admin_id: &str,
        user_id: &str,
        _form: TopicKnockAcceptedForm,
    ) -> DomainResult<()> {
        let topic = self.get_by_id(topic_id).await?;
        if !topic.owner_id.eq(admin_id) && !topic.admins.iter().any(|v| v == admin_id) {
            return Err(DomainError::Forbidden);
        }
        let Some(row) =
            topic_knock::Entity::find_by_id((topic_id.to_string(), user_id.to_string()))
                .one(&self.db)
                .await?
        else {
            return Ok(());
        };
        if row.status != "pending" {
            return Ok(());
        }

        let mut active = row.into_active_model();
        active.status = Set("accepted".to_string());
        active.admin_id = Set(admin_id.to_string());
        active.updated_at = Set(now());
        let updated = active.update(&self.db).await?;
        self.join_members(topic_id, vec![user_id.to_string()], updated.source)
            .await?;
        Ok(())
    }

    pub async fn reject_knock(
        &self,
        topic_id: &str,
        admin_id: &str,
        user_id: &str,
        _form: TopicKnockRejectedForm,
    ) -> DomainResult<()> {
        let topic = self.get_by_id(topic_id).await?;
        if !topic.owner_id.eq(admin_id) && !topic.admins.iter().any(|v| v == admin_id) {
            return Err(DomainError::Forbidden);
        }
        let Some(row) =
            topic_knock::Entity::find_by_id((topic_id.to_string(), user_id.to_string()))
                .one(&self.db)
                .await?
        else {
            return Ok(());
        };
        if row.status != "pending" {
            return Ok(());
        }
        let mut active = row.into_active_model();
        active.status = Set("rejected".to_string());
        active.admin_id = Set(admin_id.to_string());
        active.updated_at = Set(now());
        let _ = active.update(&self.db).await?;
        Ok(())
    }

    pub async fn update_notice(
        &self,
        topic_id: &str,
        admin_id: &str,
        form: UpdateNoticeForm,
    ) -> DomainResult<()> {
        let topic = self.get_by_id(topic_id).await?;
        if !topic.owner_id.eq(admin_id) && !topic.admins.iter().any(|v| v == admin_id) {
            return Err(DomainError::Forbidden);
        }
        let _ = self
            .update_topic(
                topic_id,
                OpenApiUpdateTopicForm {
                    notice: Some(crate::TopicNoticeInput {
                        text: form.text,
                        publisher: admin_id.to_string(),
                        updated_at: now(),
                    }),
                    ..OpenApiUpdateTopicForm::default()
                },
            )
            .await?;
        Ok(())
    }

    async fn modify_admin_list<F>(&self, topic_id: &str, f: F) -> DomainResult<()>
    where
        F: FnOnce(&mut Vec<String>),
    {
        let existing = topic::Entity::find_by_id(topic_id.to_string())
            .one(&self.db)
            .await?
            .ok_or(DomainError::NotFound)?;
        let mut list: Vec<String> = serde_json::from_str(&existing.admins_json).unwrap_or_default();
        f(&mut list);
        let mut active = existing.into_active_model();
        active.admins_json = Set(serde_json::to_string(&list).unwrap_or_else(|_| "[]".to_string()));
        active.updated_at = Set(now());
        let _ = active.update(&self.db).await?;
        Ok(())
    }

    async fn modify_silent_whitelist<F>(&self, topic_id: &str, f: F) -> DomainResult<Vec<String>>
    where
        F: FnOnce(&mut Vec<String>),
    {
        let existing = topic::Entity::find_by_id(topic_id.to_string())
            .one(&self.db)
            .await?
            .ok_or(DomainError::NotFound)?;
        let mut list: Vec<String> =
            serde_json::from_str(&existing.silent_white_list_json).unwrap_or_default();
        f(&mut list);
        let mut active = existing.into_active_model();
        active.silent_white_list_json =
            Set(serde_json::to_string(&list).unwrap_or_else(|_| "[]".to_string()));
        active.updated_at = Set(now());
        let _ = active.update(&self.db).await?;
        Ok(list)
    }

    async fn refresh_members_count(&self, topic_id: &str) -> DomainResult<()> {
        let count = topic_member::Entity::find()
            .filter(topic_member::Column::TopicId.eq(topic_id.to_string()))
            .count(&self.db)
            .await?;

        if let Some(existing) = topic::Entity::find_by_id(topic_id.to_string())
            .one(&self.db)
            .await?
        {
            let mut active = existing.into_active_model();
            active.members = Set(count as i32);
            active.updated_at = Set(now());
            let _ = active.update(&self.db).await?;
        }
        Ok(())
    }
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn parse_duration_to_time(duration: &str) -> Option<String> {
    let input = duration.trim().to_ascii_lowercase();
    if input.is_empty() {
        return None;
    }
    if input == "forever" {
        return Some("9999-12-31T23:59:59Z".to_string());
    }

    let now = Utc::now();
    if let Some(raw) = input.strip_suffix('m').and_then(|v| v.parse::<i64>().ok()) {
        return Some((now + Duration::minutes(raw)).to_rfc3339());
    }
    if let Some(raw) = input.strip_suffix('h').and_then(|v| v.parse::<i64>().ok()) {
        return Some((now + Duration::hours(raw)).to_rfc3339());
    }
    if let Some(raw) = input.strip_suffix('d').and_then(|v| v.parse::<i64>().ok()) {
        return Some((now + Duration::days(raw)).to_rfc3339());
    }
    None
}
