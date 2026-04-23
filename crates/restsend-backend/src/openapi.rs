use serde::{Deserialize, Serialize};

fn de_null_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_default())
}

use crate::{Conversation, Topic, User};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiErrorBody {
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiAuthForm {
    #[serde(default)]
    pub client_ip: String,
    #[serde(default)]
    pub create_when_not_exist: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiUserForm {
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub client_ip: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub timezone: String,
    #[serde(default)]
    pub avatar: String,
    #[serde(default)]
    pub gender: String,
    #[serde(default)]
    pub city: String,
    #[serde(default)]
    pub region: String,
    #[serde(default)]
    pub country: String,
    #[serde(default)]
    pub locale: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub public_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiRelationEditForm {
    pub is_contact: Option<bool>,
    #[serde(alias = "favorite")]
    pub is_star: Option<bool>,
    pub is_blocked: Option<bool>,
    pub remark: Option<String>,
    #[serde(default)]
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiUserListForm {
    #[serde(default)]
    pub user_ids: Vec<String>,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiCreateTopicForm {
    #[serde(default, alias = "ownerId")]
    pub sender_id: String,
    #[serde(default)]
    pub without_owner: bool,
    #[serde(default)]
    pub members: Vec<String>,
    #[serde(default, deserialize_with = "de_null_string")]
    pub name: String,
    #[serde(default, deserialize_with = "de_null_string")]
    pub icon: String,
    #[serde(default, deserialize_with = "de_null_string")]
    pub source: String,
    #[serde(default, deserialize_with = "de_null_string")]
    pub kind: String,
    pub multiple: Option<bool>,
    pub private: Option<bool>,
    pub knock_need_verify: Option<bool>,
    pub ensure_conversation: Option<bool>,
    pub can_override: Option<bool>,
    #[serde(default)]
    pub admins: Vec<String>,
    #[serde(default)]
    pub webhooks: Vec<String>,
    pub notice: Option<TopicNoticeInput>,
    pub extra: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TopicNoticeInput {
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub publisher: String,
    #[serde(default)]
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiUpdateTopicForm {
    #[serde(default, deserialize_with = "de_null_string")]
    pub source: String,
    #[serde(default, deserialize_with = "de_null_string")]
    pub kind: String,
    #[serde(default, deserialize_with = "de_null_string")]
    pub name: String,
    #[serde(default, deserialize_with = "de_null_string")]
    pub icon: String,
    #[serde(default)]
    pub admins: Vec<String>,
    pub private: Option<bool>,
    pub knock_need_verify: Option<bool>,
    #[serde(default)]
    pub webhooks: Vec<String>,
    pub notice: Option<TopicNoticeInput>,
    pub extra: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExtraAction {
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub key: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiUpdateTopicExtraForm {
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub actions: Vec<ExtraAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiUpdateTopicMemberForm {
    pub name: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    pub extra: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TopicKnock {
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub topic_id: String,
    #[serde(default)]
    pub user_id: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub admin_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TopicKnockForm {
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub memo: String,
}

pub type TopicKnockAcceptedForm = TopicKnockForm;
pub type TopicKnockRejectedForm = TopicKnockForm;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNoticeForm {
    #[serde(default)]
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RemoveMessagesForm {
    #[serde(default, alias = "ids")]
    pub chat_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiSilentTopicMembersForm {
    #[serde(default)]
    pub user_ids: Vec<String>,
    #[serde(default)]
    pub admin_id: String,
    #[serde(default)]
    pub duration: String,
    #[serde(default)]
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiSilentTopicForm {
    #[serde(default)]
    pub admin_id: String,
    #[serde(default)]
    pub duration: String,
    #[serde(default)]
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiUpdateConversationForm {
    pub sticky: Option<bool>,
    pub mute: Option<bool>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiPushForm {
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub chat_id: String,
    #[serde(default)]
    pub topic_id: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiChatMessageForm {
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub topic_id: String,
    #[serde(default)]
    pub attendee: String,
    #[serde(default)]
    pub chat_id: String,
    pub content: Option<crate::Content>,
    pub timeout: Option<u64>,
    #[serde(default)]
    pub e2e_content: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub source: String,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiSendChatMessageForm {
    #[serde(default)]
    pub user_ids: Vec<String>,
    #[serde(flatten)]
    pub message: OpenApiChatMessageForm,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiSendChatMessageWithFormatForm {
    #[serde(default)]
    pub user_ids: Vec<String>,
    pub message: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiSendTopicMessageForm {
    #[serde(default)]
    pub sender_id: String,
    #[serde(default)]
    pub ensure: bool,
    #[serde(default)]
    pub members: Vec<String>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub icon: String,
    #[serde(flatten)]
    pub message: OpenApiChatMessageForm,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiSendTopicMessageWithFormatForm {
    #[serde(default)]
    pub sender_id: String,
    #[serde(default)]
    pub ensure: bool,
    #[serde(default)]
    pub members: Vec<String>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub source: String,
    pub message: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ImportChatLog {
    #[serde(default)]
    pub chat_id: String,
    #[serde(default)]
    pub sender_id: String,
    pub content: Option<crate::Content>,
    #[serde(default)]
    pub source: String,
    pub seq: Option<i64>,
    #[serde(default)]
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiImportTopicMessageForm {
    #[serde(default)]
    pub messages: Vec<ImportChatLog>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChatLogSyncForm {
    pub topic_id: Option<String>,
    pub last_seq: Option<i64>,
    pub limit: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListConversationForm {
    pub offset: Option<u64>,
    pub limit: Option<u64>,
    #[serde(default)]
    pub category: String,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListConversationResult {
    pub total: i64,
    pub has_more: bool,
    pub offset: u64,
    #[serde(default)]
    pub items: Vec<crate::Conversation>,
    #[serde(default)]
    pub removed: Vec<String>,
    pub last_updated_at: Option<String>,
    pub last_removed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChatLogSyncResult {
    pub topic_id: Option<String>,
    pub has_more: bool,
    #[serde(default)]
    pub updated_at: String,
    pub last_seq: i64,
    #[serde(default)]
    pub items: Vec<crate::ChatLog>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UserOnlineResult {
    pub online: bool,
    #[serde(default)]
    pub devices: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListUserResult {
    pub has_more: bool,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub items: Vec<crate::User>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiSendMessageResponse {
    #[serde(default)]
    pub sender_id: String,
    #[serde(default)]
    pub topic_id: String,
    #[serde(default)]
    pub attendee_id: String,
    #[serde(default)]
    pub chat_id: String,
    #[serde(default)]
    pub code: i32,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub seq: i64,
    #[serde(default)]
    pub usage: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiImportTopicMessageResponse {
    #[serde(default)]
    pub chat_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum OpenApiDocSchema {
    Bool,
    String,
    StringArray,
    User,
    Topic,
    TopicMember,
    Conversation,
    UserOnlineResult,
    OpenApiSendMessageResponse,
    ChatLogSyncResult,
    Relation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiDocItem {
    pub group: String,
    pub method: String,
    pub path: String,
    pub desc: String,
    pub auth_required: bool,
    pub request: Option<OpenApiDocSchema>,
    pub response: OpenApiDocSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserPublicProfile {
    #[serde(flatten)]
    pub user: User,
    #[serde(default)]
    pub auth_token: String,
}

#[allow(dead_code)]
fn _keep_types(_: Topic, _: Conversation) {}
