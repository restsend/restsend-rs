use axum::extract::{Path, Query, State};
use axum::Json;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, ModelTrait, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::api::auth_ctx::AuthCtx;
use crate::api::error::{ApiError, ApiResult};
use crate::app::AppState;
use crate::entity::{helpdesk_inbox, helpdesk_inbox_member};
use crate::model::Conversation;
use crate::openapi::OpenApiCreateTopicForm;

// --- Request/Response types ---

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateInboxForm {
    pub name: String,
    pub display_name: Option<String>,
    pub r#type: Option<String>,
    pub greeting: Option<String>,
    pub routing_strategy: Option<String>,
    pub offline_email: Option<String>,
    pub offline_webhook_url: Option<String>,
    pub offline_webhook_secret: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInboxForm {
    pub display_name: Option<String>,
    pub r#type: Option<String>,
    pub greeting: Option<String>,
    pub greeting_enabled: Option<bool>,
    pub routing_strategy: Option<String>,
    pub offline_email: Option<String>,
    pub offline_webhook_url: Option<String>,
    pub offline_webhook_secret: Option<String>,
    pub is_active: Option<bool>,
    pub widget_config: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InboxView {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub r#type: String,
    pub widget_config: serde_json::Value,
    pub greeting: String,
    pub greeting_enabled: bool,
    pub routing_strategy: String,
    pub offline_email: String,
    pub offline_webhook_url: String,
    pub offline_webhook_secret: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<helpdesk_inbox::Model> for InboxView {
    fn from(m: helpdesk_inbox::Model) -> Self {
        let wc: serde_json::Value =
            serde_json::from_str(&m.widget_config_json).unwrap_or_default();
        Self {
            id: m.id,
            name: m.name,
            display_name: m.display_name,
            r#type: m.r#type,
            widget_config: wc,
            greeting: m.greeting,
            greeting_enabled: m.greeting_enabled,
            routing_strategy: m.routing_strategy,
            offline_email: m.offline_email,
            offline_webhook_url: m.offline_webhook_url,
            offline_webhook_secret: m.offline_webhook_secret,
            is_active: m.is_active,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InboxMemberView {
    pub inbox_id: String,
    pub user_id: String,
    pub role: String,
    pub user_name: Option<String>,
    pub user_avatar: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationListView {
    pub items: Vec<ConversationView>,
    pub total: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationView {
    pub topic_id: String,
    pub inbox_id: Option<String>,
    pub status: String,
    pub assigned_agent_id: Option<String>,
    pub assigned_agent_name: Option<String>,
    pub contact_name: Option<String>,
    pub contact_email: Option<String>,
    pub contact_avatar: Option<String>,
    pub last_message: Option<String>,
    pub unread: u64,
    pub kind: String,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListConversationsQuery {
    pub status: Option<String>,
    pub inbox_id: Option<String>,
    pub assignee_id: Option<String>,
    pub tag: Option<String>,
    pub kind: Option<String>,
    pub q: Option<String>,
    pub offset: Option<u64>,
    pub limit: Option<u64>,
}

// --- Inbox endpoints ---

pub async fn list_inboxes(
    State(state): State<AppState>,
    _auth: AuthCtx,
) -> ApiResult<Json<Vec<InboxView>>> {
    let models = helpdesk_inbox::Entity::find()
        .order_by_asc(helpdesk_inbox::Column::Name)
        .all(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let views: Vec<InboxView> = models.into_iter().map(|m| m.into()).collect();
    Ok(Json(views))
}

pub async fn create_inbox(
    State(state): State<AppState>,
    _auth: AuthCtx,
    Json(form): Json<CreateInboxForm>,
) -> ApiResult<Json<InboxView>> {
    if form.name.trim().is_empty() {
        return Err(ApiError::bad_request("name is required"));
    }
    let now = Utc::now().to_rfc3339();
    let id = format!("inbox_{}", Uuid::new_v4().simple());
    let model = helpdesk_inbox::ActiveModel {
        id: Set(id),
        name: Set(form.name),
        display_name: Set(form.display_name.unwrap_or_default()),
        r#type: Set(form.r#type.unwrap_or_else(|| "web".to_string())),
        widget_config_json: Set("{}".to_string()),
        greeting: Set(form.greeting.unwrap_or_default()),
        greeting_enabled: Set(false),
        routing_strategy: Set(form.routing_strategy.unwrap_or_else(|| "".to_string())),
        offline_email: Set(form.offline_email.unwrap_or_default()),
        offline_webhook_url: Set(form.offline_webhook_url.unwrap_or_default()),
        offline_webhook_secret: Set(form.offline_webhook_secret.unwrap_or_default()),
        is_active: Set(true),
        created_at: Set(now.clone()),
        updated_at: Set(now),
    };
    let saved = model
        .insert(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(saved.into()))
}

pub async fn get_inbox(
    State(state): State<AppState>,
    _auth: AuthCtx,
    Path(id): Path<String>,
) -> ApiResult<Json<InboxView>> {
    let model = helpdesk_inbox::Entity::find_by_id(&id)
        .one(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(model.into()))
}

pub async fn update_inbox(
    State(state): State<AppState>,
    _auth: AuthCtx,
    Path(id): Path<String>,
    Json(form): Json<UpdateInboxForm>,
) -> ApiResult<Json<InboxView>> {
    let existing = helpdesk_inbox::Entity::find_by_id(&id)
        .one(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .ok_or(ApiError::NotFound)?;

    let mut active = existing.into_active_model();
    if let Some(v) = form.display_name {
        active.display_name = Set(v);
    }
    if let Some(v) = form.r#type {
        active.r#type = Set(v);
    }
    if let Some(v) = form.greeting {
        active.greeting = Set(v);
    }
    if let Some(v) = form.greeting_enabled {
        active.greeting_enabled = Set(v);
    }
    if let Some(v) = form.routing_strategy {
        active.routing_strategy = Set(v);
    }
    if let Some(v) = form.offline_email {
        active.offline_email = Set(v);
    }
    if let Some(v) = form.offline_webhook_url {
        active.offline_webhook_url = Set(v);
    }
    if let Some(v) = form.offline_webhook_secret {
        active.offline_webhook_secret = Set(v);
    }
    if let Some(v) = form.is_active {
        active.is_active = Set(v);
    }
    if let Some(wc) = form.widget_config {
        active.widget_config_json = Set(wc.to_string());
    }
    active.updated_at = Set(Utc::now().to_rfc3339());

    let saved = active
        .update(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(saved.into()))
}

pub async fn delete_inbox(
    State(state): State<AppState>,
    _auth: AuthCtx,
    Path(id): Path<String>,
) -> ApiResult<Json<()>> {
    let model = helpdesk_inbox::Entity::find_by_id(&id)
        .one(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .ok_or(ApiError::NotFound)?;
    model
        .delete(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(()))
}

// --- Inbox members ---

pub async fn list_inbox_members(
    State(state): State<AppState>,
    _auth: AuthCtx,
    Path(inbox_id): Path<String>,
) -> ApiResult<Json<Vec<InboxMemberView>>> {
    let models = helpdesk_inbox_member::Entity::find()
        .filter(helpdesk_inbox_member::Column::InboxId.eq(&inbox_id))
        .all(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let mut views = Vec::new();
    for m in models {
        let (user_name, user_avatar) = get_user_info(&state, &m.user_id).await;
        views.push(InboxMemberView {
            inbox_id: m.inbox_id,
            user_id: m.user_id,
            role: m.role,
            user_name,
            user_avatar,
        });
    }
    Ok(Json(views))
}

pub async fn add_inbox_member(
    State(state): State<AppState>,
    _auth: AuthCtx,
    Path(inbox_id): Path<String>,
    Json(form): Json<InboxMemberView>,
) -> ApiResult<Json<InboxMemberView>> {
    let existing = helpdesk_inbox_member::Entity::find_by_id((inbox_id.clone(), form.user_id.clone()))
        .one(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    if existing.is_some() {
        return Err(ApiError::bad_request("member already exists"));
    }
    let now = Utc::now().to_rfc3339();
    let role = if form.role.is_empty() {
        "agent".to_string()
    } else {
        form.role.clone()
    };
    let model = helpdesk_inbox_member::ActiveModel {
        inbox_id: Set(inbox_id.clone()),
        user_id: Set(form.user_id.clone()),
        role: Set(role),
        created_at: Set(now),
    };
    model
        .insert(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let (user_name, user_avatar) = get_user_info(&state, &form.user_id).await;
    Ok(Json(InboxMemberView {
        inbox_id,
        user_id: form.user_id.clone(),
        role: form.role,
        user_name,
        user_avatar,
    }))
}

pub async fn remove_inbox_member(
    State(state): State<AppState>,
    _auth: AuthCtx,
    Path((inbox_id, user_id)): Path<(String, String)>,
) -> ApiResult<Json<()>> {
    let model = helpdesk_inbox_member::Entity::find_by_id((inbox_id.clone(), user_id.clone()))
        .one(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .ok_or(ApiError::NotFound)?;
    model
        .delete(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(()))
}

// --- Helpdesk Conversations ---

pub async fn list_conversations(
    State(state): State<AppState>,
    _auth: AuthCtx,
    Query(query): Query<ListConversationsQuery>,
) -> ApiResult<Json<ConversationListView>> {
    use crate::entity::topic;
    let limit = query.limit.unwrap_or(50).min(200);
    let offset = query.offset.unwrap_or(0);

    let mut cond = sea_orm::Condition::all()
        .add(topic::Column::Kind.eq("helpdesk"))
        .add(topic::Column::Enabled.eq(true));

    if let Some(inbox_id) = &query.inbox_id {
        // For helpdesk topics, extra_json contains {"inbox_id": "..."}
        // Since we can't easily filter on JSON content, we filter by prefix
        // A more robust approach would be to add an inbox_id column to topics
        // For now, we filter in-memory after query
    }

    let mut find = topic::Entity::find()
        .filter(cond)
        .order_by_desc(topic::Column::UpdatedAt);

    let total = find
        .clone()
        .count(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    let models = find
        .offset(offset)
        .limit(limit)
        .all(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    let mut items: Vec<ConversationView> = Vec::new();
    for m in models {
        let extra: HashMap<String, String> =
            crate::entity::decode_json(&m.extra_json);
        let inbox_id = extra.get("inbox_id").cloned();
        let status = extra
            .get("status")
            .cloned()
            .unwrap_or_else(|| "open".to_string());
        let assigned = extra.get("assigned_agent_id").cloned();

        // filter by inbox_id if specified
        if let Some(ref filter_inbox) = query.inbox_id {
            if inbox_id.as_deref() != Some(filter_inbox.as_str()) {
                continue;
            }
        }
        // filter by assignee
        if let Some(ref aid) = query.assignee_id {
            if assigned.as_deref() != Some(aid.as_str()) {
                continue;
            }
        }
        // filter by status
        if let Some(ref s) = query.status {
            if status != *s {
                continue;
            }
        }
        // filter by tag
        if let Some(ref tag) = query.tag {
            let tags: Vec<String> = extra
                .get("tags")
                .map(|t| t.split(',').map(|s| s.to_string()).collect())
                .unwrap_or_default();
            if !tags.contains(tag) {
                continue;
            }
        }
        // filter by kind
        if let Some(ref k) = query.kind {
            let topic_kind = if m.multiple { "multiple" } else { "single" };
            if topic_kind != k.as_str() && k != "all" {
                continue;
            }
        }

        let assigned_name = assigned
            .as_deref()
            .and_then(|id| get_user_info_sync(&state, id));

        let contact_name = extra.get("contact_name").cloned();
        let contact_email = extra.get("contact_email").cloned();
        let contact_avatar = extra.get("contact_avatar").cloned();
        let tags_str: Vec<String> = extra
            .get("tags")
            .map(|t| t.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
            .unwrap_or_default();

        items.push(ConversationView {
            topic_id: m.id.clone(),
            inbox_id,
            status,
            assigned_agent_id: assigned,
            assigned_agent_name: assigned_name,
            contact_name,
            contact_email,
            contact_avatar,
            last_message: None,
            unread: 0,
            kind: if m.multiple { "multiple".to_string() } else { "single".to_string() },
            tags: tags_str,
            created_at: m.created_at,
            updated_at: m.updated_at,
        });
    }

    Ok(Json(ConversationListView { items, total }))
}

pub async fn get_conversation(
    State(state): State<AppState>,
    _auth: AuthCtx,
    Path(topic_id): Path<String>,
) -> ApiResult<Json<ConversationView>> {
    use crate::entity::topic;
    let m = topic::Entity::find_by_id(&topic_id)
        .one(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .ok_or(ApiError::NotFound)?;

    if m.kind != "helpdesk" {
        return Err(ApiError::NotFound);
    }
    let extra: HashMap<String, String> = crate::entity::decode_json(&m.extra_json);
    let inbox_id = extra.get("inbox_id").cloned();
    let status = extra
        .get("status")
        .cloned()
        .unwrap_or_else(|| "open".to_string());
    let assigned = extra.get("assigned_agent_id").cloned();
    let assigned_name = assigned
        .as_deref()
        .and_then(|id| get_user_info_sync(&state, id));
    let tags_str: Vec<String> = extra
        .get("tags")
        .map(|t| t.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
        .unwrap_or_default();

    Ok(Json(ConversationView {
        topic_id: m.id,
        inbox_id,
        status,
        assigned_agent_id: assigned,
        assigned_agent_name: assigned_name,
        contact_name: extra.get("contact_name").cloned(),
        contact_email: extra.get("contact_email").cloned(),
        contact_avatar: extra.get("contact_avatar").cloned(),
        last_message: None,
        unread: 0,
        kind: if m.multiple { "multiple".to_string() } else { "single".to_string() },
        tags: tags_str,
        created_at: m.created_at,
        updated_at: m.updated_at,
    }))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateConversationStatusForm {
    pub status: String,
}

pub async fn update_conversation_status(
    State(state): State<AppState>,
    _auth: AuthCtx,
    Path(topic_id): Path<String>,
    Json(form): Json<UpdateConversationStatusForm>,
) -> ApiResult<Json<()>> {
    use crate::entity::topic;
    let m = topic::Entity::find_by_id(&topic_id)
        .one(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .ok_or(ApiError::NotFound)?;

    if m.kind != "helpdesk" {
        return Err(ApiError::NotFound);
    }

    let mut extra: HashMap<String, String> = crate::entity::decode_json(&m.extra_json);
    extra.insert("status".to_string(), form.status);

    let mut active = m.into_active_model();
    active.extra_json = Set(serde_json::to_string(&extra).unwrap_or_default());
    active.updated_at = Set(Utc::now().to_rfc3339());
    active
        .update(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(()))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignConversationForm {
    pub agent_id: Option<String>,
}

pub async fn assign_conversation(
    State(state): State<AppState>,
    _auth: AuthCtx,
    Path(topic_id): Path<String>,
    Json(form): Json<AssignConversationForm>,
) -> ApiResult<Json<()>> {
    use crate::entity::topic;
    let m = topic::Entity::find_by_id(&topic_id)
        .one(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .ok_or(ApiError::NotFound)?;

    if m.kind != "helpdesk" {
        return Err(ApiError::NotFound);
    }

    let mut extra: HashMap<String, String> = crate::entity::decode_json(&m.extra_json);
    if let Some(agent_id) = &form.agent_id {
        extra.insert("assigned_agent_id".to_string(), agent_id.clone());
    } else {
        extra.remove("assigned_agent_id");
    }

    let mut active = m.into_active_model();
    active.extra_json = Set(serde_json::to_string(&extra).unwrap_or_default());
    active.updated_at = Set(Utc::now().to_rfc3339());
    active
        .update(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(()))
}

// --- Conversation messages ---

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationMessagesView {
    pub items: Vec<crate::ChatLog>,
    pub total: u64,
    pub has_more: bool,
}

pub async fn get_conversation_messages(
    State(state): State<AppState>,
    _auth: AuthCtx,
    Path(topic_id): Path<String>,
    Query(query): Query<ListConversationMessagesQuery>,
) -> ApiResult<Json<ConversationMessagesView>> {
    use crate::entity::chat_log;
    let limit = query.limit.unwrap_or(50).min(200);
    let offset = query.offset.unwrap_or(0);
    let last_seq = query.last_seq.unwrap_or(0);

    let mut cond = chat_log::Column::TopicId.eq(&topic_id);
    if last_seq > 0 {
        cond = cond.add(chat_log::Column::Seq.gt(last_seq));
    }

    let total = chat_log::Entity::find()
        .filter(cond.clone())
        .count(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    let models = chat_log::Entity::find()
        .filter(cond)
        .order_by_desc(chat_log::Column::Seq)
        .offset(offset)
        .limit(limit)
        .all(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    let has_more = total as u64 > offset + limit;
    let items: Vec<crate::ChatLog> = models
        .into_iter()
        .map(|m| -> crate::ChatLog { m.into() })
        .collect();
    Ok(Json(ConversationMessagesView {
        items,
        total,
        has_more,
    }))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListConversationMessagesQuery {
    pub last_seq: Option<i64>,
    pub offset: Option<u64>,
    pub limit: Option<u64>,
}

// --- Live Chat ---

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartLiveChatForm {
    pub inbox_id: Option<String>,
    pub guest_id: Option<String>,
    pub display_name: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartLiveChatResponse {
    pub topic_id: String,
    pub user_id: String,
    pub token: String,
    pub display_name: String,
}

pub async fn start_livechat(
    State(state): State<AppState>,
    Json(form): Json<StartLiveChatForm>,
) -> ApiResult<Json<StartLiveChatResponse>> {
    let guest_id = form.guest_id.unwrap_or_else(|| {
        format!("guest_{}", Uuid::new_v4().simple())
    });

    let display_name = form.display_name.unwrap_or_else(|| guest_id.clone());

    let _ = match state.user_service.get_by_user_id(&guest_id).await {
        Ok(user) => user,
        Err(_) => {
            state
                .user_service
                .register(
                    &guest_id,
                    crate::OpenApiUserForm {
                        display_name: display_name.clone(),
                        source: "guest".to_string(),
                        ..crate::OpenApiUserForm::default()
                    },
                )
                .await
                .map_err(|e| ApiError::internal(e.to_string()))?
        }
    };

    let token = state
        .auth_service
        .issue_token(&guest_id)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    let inbox_id = form.inbox_id.unwrap_or_else(|| "inbox_default".to_string());
    let mut extra = HashMap::new();
    extra.insert("inbox_id".to_string(), inbox_id.clone());
    extra.insert("source".to_string(), "livechat".to_string());
    extra.insert("status".to_string(), "open".to_string());
    extra.insert("contact_name".to_string(), display_name.clone());

    let topic_form = OpenApiCreateTopicForm {
        sender_id: guest_id.clone(),
        kind: "helpdesk".to_string(),
        name: display_name.clone(),
        multiple: Some(false),
        private: Some(true),
        ensure_conversation: Some(true),
        extra: Some(extra),

        ..OpenApiCreateTopicForm::default()
    };

    let topic = state
        .topic_service
        .create_topic(None, topic_form)
        .await
        .map_err(|e| ApiError::internal(format!("create topic failed: {}", e)))?;

    let now = Utc::now().to_rfc3339();
    let conv = Conversation {
        owner_id: guest_id.clone(),
        topic_id: topic.id.clone(),
        name: display_name.clone(),
        kind: "helpdesk".to_string(),
        members: 1,
        source: "helpdesk".to_string(),
        updated_at: now,
        ..Conversation::default()
    };
    let _ = state.conversation_service.create_or_update(conv).await;

    tracing::info!(
        guest_id = %guest_id,
        topic_id = %topic.id,
        inbox_id = %inbox_id,
        "livechat session created"
    );

    Ok(Json(StartLiveChatResponse {
        topic_id: topic.id,
        user_id: guest_id,
        token,
        display_name,
    }))
}

// --- Canned Responses ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CannedResponseView {
    pub id: String,
    pub shortcut: String,
    pub content: String,
    pub inbox_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCannedResponseForm {
    pub shortcut: String,
    pub content: String,
    pub inbox_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCannedResponseForm {
    pub shortcut: Option<String>,
    pub content: Option<String>,
    pub inbox_id: Option<String>,
}

// For now, store canned responses in a simple in-memory structure
use std::sync::{Mutex, OnceLock};

fn canned_responses() -> &'static Mutex<Vec<CannedResponseView>> {
    static STORE: OnceLock<Mutex<Vec<CannedResponseView>>> = OnceLock::new();
    STORE.get_or_init(|| {
        Mutex::new(vec![
            CannedResponseView {
                id: "cr_hi".to_string(),
                shortcut: "hi".to_string(),
                content: "您好！欢迎来到在线客服，请问有什么可以帮助您的？".to_string(),
                inbox_id: None,
            },
            CannedResponseView {
                id: "cr_bye".to_string(),
                shortcut: "bye".to_string(),
                content: "感谢您的咨询，祝您生活愉快！".to_string(),
                inbox_id: None,
            },
        ])
    })
}

pub async fn list_canned_responses(
    State(_state): State<AppState>,
    _auth: AuthCtx,
) -> ApiResult<Json<Vec<CannedResponseView>>> {
    let list = canned_responses().lock().unwrap().clone();
    Ok(Json(list))
}

pub async fn create_canned_response(
    State(_state): State<AppState>,
    _auth: AuthCtx,
    Json(form): Json<CreateCannedResponseForm>,
) -> ApiResult<Json<CannedResponseView>> {
    if form.shortcut.trim().is_empty() || form.content.trim().is_empty() {
        return Err(ApiError::bad_request("shortcut and content are required"));
    }
    let view = CannedResponseView {
        id: format!("cr_{}", Uuid::new_v4().simple()),
        shortcut: form.shortcut,
        content: form.content,
        inbox_id: form.inbox_id,
    };
    canned_responses().lock().unwrap().push(view.clone());
    Ok(Json(view))
}

pub async fn update_canned_response(
    State(_state): State<AppState>,
    _auth: AuthCtx,
    Path(id): Path<String>,
    Json(form): Json<UpdateCannedResponseForm>,
) -> ApiResult<Json<CannedResponseView>> {
    let mut list = canned_responses().lock().unwrap();
    let idx = list.iter().position(|c| c.id == id).ok_or(ApiError::NotFound)?;
    let item = &mut list[idx];
    if let Some(v) = form.shortcut {
        item.shortcut = v;
    }
    if let Some(v) = form.content {
        item.content = v;
    }
    if let Some(v) = form.inbox_id {
        item.inbox_id = Some(v);
    }
    Ok(Json(list[idx].clone()))
}

pub async fn delete_canned_response(
    State(_state): State<AppState>,
    _auth: AuthCtx,
    Path(id): Path<String>,
) -> ApiResult<Json<()>> {
    let mut list = canned_responses().lock().unwrap();
    let idx = list.iter().position(|c| c.id == id).ok_or(ApiError::NotFound)?;
    list.remove(idx);
    Ok(Json(()))
}

// --- Labels ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabelView {
    pub id: String,
    pub name: String,
    pub color: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateLabelForm {
    pub name: String,
    pub color: String,
}

fn labels() -> &'static Mutex<Vec<LabelView>> {
    static STORE: OnceLock<Mutex<Vec<LabelView>>> = OnceLock::new();
    STORE.get_or_init(|| {
        Mutex::new(vec![
            LabelView {
                id: "label_problem".to_string(),
                name: "问题".to_string(),
                color: "#ef4444".to_string(),
            },
            LabelView {
                id: "label_suggestion".to_string(),
                name: "建议".to_string(),
                color: "#3b82f6".to_string(),
            },
            LabelView {
                id: "label_return".to_string(),
                name: "退货".to_string(),
                color: "#f59e0b".to_string(),
            },
        ])
    })
}

pub async fn list_labels(
    State(_state): State<AppState>,
    _auth: AuthCtx,
) -> ApiResult<Json<Vec<LabelView>>> {
    let list = labels().lock().unwrap().clone();
    Ok(Json(list))
}

pub async fn create_label(
    State(_state): State<AppState>,
    _auth: AuthCtx,
    Json(form): Json<CreateLabelForm>,
) -> ApiResult<Json<LabelView>> {
    if form.name.trim().is_empty() {
        return Err(ApiError::bad_request("name is required"));
    }
    let view = LabelView {
        id: format!("label_{}", Uuid::new_v4().simple()),
        name: form.name,
        color: if form.color.is_empty() {
            "#6b7280".to_string()
        } else {
            form.color
        },
    };
    labels().lock().unwrap().push(view.clone());
    Ok(Json(view))
}

pub async fn delete_label(
    State(_state): State<AppState>,
    _auth: AuthCtx,
    Path(id): Path<String>,
) -> ApiResult<Json<()>> {
    let mut list = labels().lock().unwrap();
    let idx = list.iter().position(|l| l.id == id).ok_or(ApiError::NotFound)?;
    list.remove(idx);
    Ok(Json(()))
}

// --- Inbox Settings ---

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InboxSettingsView {
    pub inbox_id: String,
    pub routing_strategy: String,
    pub offline_email: String,
    pub offline_webhook_url: String,
    pub offline_webhook_secret: String,
    pub offline_notification_preview: String,
}

pub async fn get_inbox_settings(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(inbox_id): Path<String>,
) -> ApiResult<Json<InboxSettingsView>> {
    auth.ensure_staff()?;
    let inbox = helpdesk_inbox::Entity::find_by_id(&inbox_id)
        .one(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .ok_or(ApiError::NotFound)?;

    let preview = generate_offline_preview(&inbox);

    Ok(Json(InboxSettingsView {
        inbox_id: inbox.id,
        routing_strategy: inbox.routing_strategy,
        offline_email: inbox.offline_email,
        offline_webhook_url: inbox.offline_webhook_url,
        offline_webhook_secret: inbox.offline_webhook_secret,
        offline_notification_preview: preview,
    }))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInboxSettingsForm {
    pub routing_strategy: Option<String>,
    pub offline_email: Option<String>,
    pub offline_webhook_url: Option<String>,
    pub offline_webhook_secret: Option<String>,
}

pub async fn update_inbox_settings(
    State(state): State<AppState>,
    auth: AuthCtx,
    Path(inbox_id): Path<String>,
    Json(form): Json<UpdateInboxSettingsForm>,
) -> ApiResult<Json<InboxSettingsView>> {
    auth.ensure_staff()?;
    let existing = helpdesk_inbox::Entity::find_by_id(&inbox_id)
        .one(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .ok_or(ApiError::NotFound)?;

    let mut active = existing.into_active_model();
    if let Some(v) = form.routing_strategy {
        active.routing_strategy = Set(v);
    }
    if let Some(v) = form.offline_email {
        active.offline_email = Set(v);
    }
    if let Some(v) = form.offline_webhook_url {
        active.offline_webhook_url = Set(v);
    }
    if let Some(v) = form.offline_webhook_secret {
        active.offline_webhook_secret = Set(v);
    }
    active.updated_at = Set(Utc::now().to_rfc3339());

    let saved = active
        .update(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    let preview = generate_offline_preview(&saved);

    Ok(Json(InboxSettingsView {
        inbox_id: saved.id,
        routing_strategy: saved.routing_strategy,
        offline_email: saved.offline_email,
        offline_webhook_url: saved.offline_webhook_url,
        offline_webhook_secret: saved.offline_webhook_secret,
        offline_notification_preview: preview,
    }))
}

// --- Helpers ---

async fn get_user_info(state: &AppState, user_id: &str) -> (Option<String>, Option<String>) {
    match state.user_service.get_any_by_user_id(user_id).await {
        Ok(u) => (Some(u.name), Some(u.avatar)),
        Err(_) => (None, None),
    }
}

fn get_user_info_sync(state: &AppState, user_id: &str) -> Option<String> {
    let rt = tokio::runtime::Handle::try_current();
    match rt {
        Ok(handle) => {
            let state = state.clone();
            let uid = user_id.to_string();
            match handle.block_on(async move {
                state.user_service.get_any_by_user_id(&uid).await
            }) {
                Ok(u) => Some(u.name),
                Err(_) => None,
            }
        }
        Err(_) => None,
    }
}

fn generate_offline_preview(inbox: &helpdesk_inbox::Model) -> String {
    let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    format!(
        r#"{{"event":"offline_message","inbox":"{}","timestamp":"{}","contact":{{"name":"访客","email":""}},"message":"访客发送了一条消息，但当前无在线客服。"}}"#,
        inbox.name, now
    )
}
