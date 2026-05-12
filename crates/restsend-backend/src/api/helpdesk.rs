use axum::extract::{Path, State};
use axum::Json;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::api::auth_ctx::AuthCtx;
use crate::api::error::{ApiError, ApiResult};
use crate::app::AppState;
use crate::model::Conversation;
use crate::openapi::OpenApiCreateTopicForm;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateInboxForm {
    pub name: String,
    pub display_name: Option<String>,
    pub r#type: Option<String>,
    pub greeting: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InboxView {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub r#type: String,
    pub is_active: bool,
    pub created_at: String,
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
    pub contact_name: Option<String>,
    pub contact_email: Option<String>,
    pub last_message: Option<String>,
    pub unread: u64,
    pub created_at: String,
    pub updated_at: String,
}

// --- Inbox endpoints ---

pub async fn list_inboxes(
    State(_state): State<AppState>,
    _auth: AuthCtx,
) -> ApiResult<Json<Vec<InboxView>>> {
    let inboxes = vec![
        InboxView {
            id: "inbox_default".to_string(),
            name: "default".to_string(),
            display_name: "默认 Inbox".to_string(),
            r#type: "web".to_string(),
            is_active: true,
            created_at: chrono::Utc::now().to_rfc3339(),
        },
    ];
    Ok(Json(inboxes))
}

pub async fn create_inbox(
    State(_state): State<AppState>,
    _auth: AuthCtx,
    Json(_form): Json<CreateInboxForm>,
) -> ApiResult<Json<InboxView>> {
    Err(ApiError::not_implemented("not implemented"))
}

pub async fn get_inbox(
    State(_state): State<AppState>,
    _auth: AuthCtx,
    Path(_id): Path<String>,
) -> ApiResult<Json<InboxView>> {
    Err(ApiError::not_implemented("not implemented"))
}

pub async fn update_inbox(
    State(_state): State<AppState>,
    _auth: AuthCtx,
    Path(_id): Path<String>,
    Json(_form): Json<CreateInboxForm>,
) -> ApiResult<Json<InboxView>> {
    Err(ApiError::not_implemented("not implemented"))
}

pub async fn delete_inbox(
    State(_state): State<AppState>,
    _auth: AuthCtx,
    Path(_id): Path<String>,
) -> ApiResult<Json<()>> {
    Err(ApiError::not_implemented("not implemented"))
}

// --- Inbox members ---

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InboxMemberView {
    pub inbox_id: String,
    pub user_id: String,
    pub role: String,
}

pub async fn list_inbox_members(
    State(_state): State<AppState>,
    _auth: AuthCtx,
    Path(_inbox_id): Path<String>,
) -> ApiResult<Json<Vec<InboxMemberView>>> {
    Ok(Json(vec![]))
}

pub async fn add_inbox_member(
    State(_state): State<AppState>,
    _auth: AuthCtx,
    Path(_inbox_id): Path<String>,
    Json(_form): Json<InboxMemberView>,
) -> ApiResult<Json<InboxMemberView>> {
    Err(ApiError::not_implemented("not implemented"))
}

pub async fn remove_inbox_member(
    State(_state): State<AppState>,
    _auth: AuthCtx,
    Path((_inbox_id, _user_id)): Path<(String, String)>,
) -> ApiResult<Json<()>> {
    Err(ApiError::not_implemented("not implemented"))
}

// --- Helpdesk Conversations ---

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListConversationsQuery {
    pub status: Option<String>,
    pub inbox_id: Option<String>,
    pub assignee_id: Option<String>,
    pub q: Option<String>,
    pub offset: Option<u64>,
    pub limit: Option<u64>,
}

pub async fn list_conversations(
    State(_state): State<AppState>,
    _auth: AuthCtx,
) -> ApiResult<Json<ConversationListView>> {
    Ok(Json(ConversationListView {
        items: vec![],
        total: 0,
    }))
}

pub async fn get_conversation(
    State(_state): State<AppState>,
    _auth: AuthCtx,
    Path(_topic_id): Path<String>,
) -> ApiResult<Json<ConversationView>> {
    Err(ApiError::not_implemented("not implemented"))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateConversationStatusForm {
    pub status: String,
}

pub async fn update_conversation_status(
    State(_state): State<AppState>,
    _auth: AuthCtx,
    Path(_topic_id): Path<String>,
    Json(_form): Json<UpdateConversationStatusForm>,
) -> ApiResult<Json<()>> {
    Err(ApiError::not_implemented("not implemented"))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignConversationForm {
    pub agent_id: Option<String>,
}

pub async fn assign_conversation(
    State(_state): State<AppState>,
    _auth: AuthCtx,
    Path(_topic_id): Path<String>,
    Json(_form): Json<AssignConversationForm>,
) -> ApiResult<Json<()>> {
    Err(ApiError::not_implemented("not implemented"))
}

// --- Live Chat ---

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartLiveChatForm {
    pub inbox_id: Option<String>,
    /// Optional guest user ID. Auto-generated if not provided.
    pub guest_id: Option<String>,
    /// Optional display name for the guest
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
    // 1. Generate or use provided guest_id
    let guest_id = form.guest_id.unwrap_or_else(|| {
        format!("guest_{}", uuid::Uuid::new_v4().simple())
    });

    let display_name = form.display_name.unwrap_or_else(|| guest_id.clone());

    // 2. Register guest user if not exists
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

    // 3. Issue auth token
    let token = state
        .auth_service
        .issue_token(&guest_id)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    // 4. Create helpdesk topic
    let inbox_id = form.inbox_id.unwrap_or_else(|| "inbox_default".to_string());
    let mut extra = HashMap::new();
    extra.insert("inbox_id".to_string(), inbox_id.clone());
    extra.insert("source".to_string(), "livechat-page".to_string());
    extra.insert("status".to_string(), "open".to_string());

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

    // 5. Ensure the owner has a conversation
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CannedResponseView {
    pub id: String,
    pub shortcut: String,
    pub content: String,
    pub inbox_id: Option<String>,
}

pub async fn list_canned_responses(
    State(_state): State<AppState>,
    _auth: AuthCtx,
) -> ApiResult<Json<Vec<CannedResponseView>>> {
    Ok(Json(vec![]))
}

// --- Labels ---

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabelView {
    pub id: String,
    pub name: String,
    pub color: String,
}

pub async fn list_labels(
    State(_state): State<AppState>,
    _auth: AuthCtx,
) -> ApiResult<Json<Vec<LabelView>>> {
    Ok(Json(vec![]))
}
