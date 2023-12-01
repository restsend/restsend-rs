use std::collections::HashMap;

use super::{
    Client, CONVERSATIONS_SYNC, CONVERSATION_CACHE, KEY_CONVERSATIONS_SYNC_AT, LIMIT, USER_CACHE,
};
use crate::error::ClientError;
use crate::models::{
    AuthInfo, ChatLog, Conversation, ListChatLogResult, ListConversationResult, ListUserResult,
    Topic, TopicKnock, TopicMember, TopicNotice, User,
};
use crate::utils::is_expired;
use anyhow::Result;
use log::{debug, warn};

use reqwest::header::HeaderValue;
use tokio::time::Duration;

#[derive(serde::Deserialize, Default)]
pub struct UserProfile {
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
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResp {
    pub email: String,
    #[serde(default)]
    pub display_name: String,
    pub token: String,
    #[serde(default)]
    pub profile: UserProfile,
}

#[derive(serde::Deserialize)]
pub struct CommonResp {
    #[allow(unused)]
    ok: bool,
}

pub fn login(endpoint: String, email: String, password: String) -> Result<AuthInfo> {
    let data = serde_json::json!({
        "email": email,
        "password": password,
        "remember": true,
    });
    let url = format!("{}/auth/login", endpoint);
    let req = reqwest::ClientBuilder::new()
        .user_agent(crate::USER_AGENT)
        .build()?
        .post(&url)
        .header(
            reqwest::header::CONTENT_TYPE,
            HeaderValue::from_bytes(b"application/json").unwrap(),
        )
        .body(data.to_string())
        .timeout(Duration::from_secs(super::API_TIMEOUT_SECS));

    warn!("login url:{} email:{}", url, email);

    let r = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let resp = req.send().await?;
            match resp.status() {
                reqwest::StatusCode::OK => {
                    let resp: crate::client::services::LoginResp = resp.json().await?;
                    Ok(AuthInfo {
                        endpoint,
                        user_id: resp.email,
                        avatar: resp.profile.avatar,
                        name: resp.display_name,
                        token: resp.token,
                    })
                }
                _ => {
                    let err = resp.text().await?;
                    Err(ClientError::HTTPError(err))
                }
            }
        });
    match r {
        Ok(info) => Ok(info),
        Err(e) => {
            warn!("login with {} failed: {}", email, e);
            Err(e)
        }
    }
}

impl Client {
    pub fn send_http_request<R>(
        &self,
        path: &str,
        value: serde_json::Value,
        timeout_secs: Option<u64>,
        method: http::Method,
    ) -> Result<R>
    where
        R: serde::de::DeserializeOwned,
    {
        // 可以并发发送请求
        let full_url = self.net_store.endpoint()? + path;
        let req = self.net_store.make_request(
            method,
            &full_url,
            None,
            value.to_string(),
            timeout_secs.unwrap_or(super::API_TIMEOUT_SECS),
        )?;

        let (tx, rx) = tokio::sync::oneshot::channel::<String>();
        let runner = async move {
            let resp = req.send().await?;
            let body = match resp.status() {
                reqwest::StatusCode::OK => Ok(resp.text().await?),
                _ => {
                    let status = resp.status().as_u16();
                    let reason = resp.text().await?;
                    Err(ClientError::HTTPError(format! {"{} {}", status, reason}))
                }
            }?;
            tx.send(body)
                .map_err(|e| ClientError::HTTPError(e.to_string()))
        };

        let body = tokio::task::block_in_place(move || -> Result<String> {
            self.runtime.spawn(runner);
            rx.blocking_recv()
                .map_err(|e| ClientError::HTTPError(e.to_string()))
        })?;

        debug!("send_http_request resp body: {} -> {:.256}", full_url, body);
        serde_json::from_str::<R>(&body).map_err(|e| e.into())
    }

    pub fn json_get<R>(&self, path: &str, timeout_secs: Option<u64>) -> Result<R>
    where
        R: serde::de::DeserializeOwned,
    {
        self.send_http_request(
            path,
            serde_json::Value::Null,
            timeout_secs,
            http::Method::GET,
        )
    }

    pub fn json_call<R>(
        &self,
        path: &str,
        value: serde_json::Value,
        timeout_secs: Option<u64>,
    ) -> Result<R>
    where
        R: serde::de::DeserializeOwned,
    {
        self.send_http_request(path, value, timeout_secs, http::Method::POST)
    }

    pub fn attach(&self, info: AuthInfo) -> Result<()> {
        self.db.set_value(super::KEY_TOKEN, &info.token)?;
        self.db.set_value(super::KEY_USER_ID, &info.user_id)?;

        self.net_store.set_me(info.user_id);
        self.net_store.set_auth_token(&info.token);
        Ok(self.ctrl_tx.send(super::CtrlMessageType::Connect)?)
    }

    pub fn logout(&self) -> Result<()> {
        self.json_get::<HashMap<String, String>>("/auth/logout", None)?;
        self.db.set_value(super::KEY_TOKEN, "")
    }

    pub fn get_user(&self, user_id: String) -> Result<User> {
        if let Ok(cached_user) = self.db.get_user(&user_id) {
            if !is_expired(&cached_user.cached_at, USER_CACHE) {
                return Ok(cached_user);
            }
        }
        self.json_call::<User>(
            &format!("/api/profile/{}", user_id),
            serde_json::json!({}),
            None,
        )
        .and_then(|mut user| {
            user.cached_at = chrono::Utc::now().to_rfc3339();
            if !user.avatar.is_empty() && !user.avatar.starts_with("http") {
                let endpoint = self.net_store.endpoint()?;
                user.avatar = format!(
                    "{}{}",
                    endpoint.trim_end_matches('/').to_string(),
                    user.avatar
                );
            }
            self.db.save_user(&user)?;
            Ok(user)
        })
    }
    pub fn set_user_block(&self, user_id: String, block: bool) -> Result<()> {
        let action = if block { "block" } else { "unblock" };
        self.json_call::<bool>(
            &format!("/api/{}/{}", action, user_id),
            serde_json::json!({}),
            None,
        )
        .and(self.db.set_user_block(&user_id, block))
        .and(Ok(()))
    }

    pub fn get_conversation(&self, topic_id: String) -> Result<Conversation> {
        if let Ok(cached_conversation) = self.db.get_conversation(&topic_id) {
            if !is_expired(&cached_conversation.cached_at, CONVERSATION_CACHE) {
                return Ok(cached_conversation);
            }
        }

        self.json_call(
            &format!("/api/chat/info/{}", topic_id),
            serde_json::json!({}),
            None,
        )
        .and_then(|mut c: Conversation| {
            c.cached_at = chrono::Utc::now().to_rfc3339();

            self.db.save_conversation(&c)?;
            Ok(c)
        })
    }

    pub fn remove_conversation(&self, topic_id: String) -> Result<()> {
        self.json_call::<CommonResp>(
            &format!("/api/chat/remove/{}", topic_id),
            serde_json::json!({}),
            None,
        )
        .and_then(|_| self.db.remove_conversation(&topic_id))?;

        if let Some(cb) = self.callback.read().unwrap().as_ref() {
            cb.on_conversation_removed(topic_id);
        }
        Ok(())
    }

    fn update_conversation(&self, topic_id: &str, data: &serde_json::Value) -> Result<()> {
        self.json_call::<CommonResp>(
            &format!("/api/chat/update/{}", topic_id),
            data.clone(),
            None,
        )
        .and(Ok(()))
    }

    //置顶会话
    pub fn set_conversation_sticky(&self, topic_id: String, sticky: bool) -> Result<()> {
        let mut data = serde_json::json!({});
        data["sticky"] = sticky.into();
        self.update_conversation(&topic_id, &data)?;
        self.db.set_conversation_sticky(&topic_id, sticky)
    }

    pub fn set_conversation_mute(&self, topic_id: String, mute: bool) -> Result<()> {
        let mut data = serde_json::json!({});
        data["mute"] = mute.into();
        self.update_conversation(&topic_id, &data)?;
        self.db.set_conversation_mute(&topic_id, mute)
    }

    // 获取会话列表, updated_at是上次更新的时间, 如果为空, 则返回所有的会话
    pub fn get_conversations(
        &self,
        updated_at: String,
        limit: u32,
    ) -> Result<ListConversationResult> {
        let mut data = serde_json::json!({ "limit": limit });
        if !updated_at.is_empty() {
            data["updatedAt"] = updated_at.into();
        }
        let mut lr: ListConversationResult = self.json_call("/api/chat/list", data, None)?;
        for c in lr.items.iter_mut() {
            c.cached_at = chrono::Utc::now().to_rfc3339();
            self.db.save_conversation(c)?;
        }
        Ok(lr)
    }

    pub fn get_topic(&self, topic_id: String) -> Result<Topic> {
        if let Ok(cached_topic) = self.db.get_topic(&topic_id) {
            if !is_expired(&cached_topic.cached_at, CONVERSATION_CACHE) {
                return Ok(cached_topic);
            }
        }
        self.json_call(
            &format!("/api/topic/info/{}", topic_id),
            serde_json::json!({}),
            None,
        )
        .and_then(|mut topic: Topic| {
            topic.cached_at = chrono::Utc::now().to_rfc3339();
            if !topic.icon.is_empty() && !topic.icon.starts_with("http") {
                let endpoint = self.net_store.endpoint()?;
                topic.icon = format!(
                    "{}{}",
                    endpoint.trim_end_matches('/').to_string(),
                    topic.icon
                );
            }

            self.db.save_topic(&topic).and(Ok(topic))
        })
    }

    pub fn get_topic_admins(&self, topic_id: String) -> Result<Vec<User>> {
        let admins = self.db.get_topic_admins(&topic_id)?;
        if admins.is_empty() {
            return Ok(vec![]);
        }
        let data = serde_json::json!({ "ids": admins });
        self.json_call("/api/profile", data, None)
    }

    pub fn get_topic_members(
        &self,
        topic_id: String,
        updated_at: String,
        limit: u32,
    ) -> Result<ListUserResult> {
        let mut data = serde_json::json!({
            "topicId": topic_id,
            "limit": limit
        });

        if !updated_at.is_empty() {
            data["updatedAt"] = updated_at.into();
        }
        let mut lr: ListUserResult =
            self.json_call(&format!("/api/topic/members/{}", topic_id), data, None)?;
        let topic = self.db.get_topic(&topic_id)?;
        lr.items
            .iter_mut()
            .map(|u| {
                u.cached_at = chrono::Utc::now().to_rfc3339();
                let mut topic_member = TopicMember::new(&topic_id, &u.user_id);
                topic_member.is_admin = topic.admins.contains(&u.user_id);
                topic_member.is_owner = topic.owner_id == u.user_id;
                topic_member.remark = u.remark.clone();
                topic_member.cached_at = u.cached_at.clone();
                (u, topic_member)
            })
            .try_for_each(|(u, topic_member)| -> Result<()> {
                self.db.save_user(u)?;
                self.db.save_topic_member(&topic_member)?;
                Ok(())
            })?;

        lr.removed.iter().try_for_each(|u| -> Result<()> {
            self.db.remove_topic_member(&topic_id, &u)?;
            Ok(())
        })?;
        Ok(lr)
    }

    pub fn get_topic_owner(&self, topic_id: String) -> Result<User> {
        let owner_id = self.db.get_topic_owner(&topic_id)?;
        self.get_user(owner_id)
    }

    pub fn create_topic(&self, name: String, icon: String, members: Vec<String>) -> Result<Topic> {
        let data = serde_json::json!({
            "name": name,
            "icon": icon,
            "members": members
        });
        self.json_call("/api/topic/create", data, None)
            .and_then(|topic: Topic| self.db.save_topic(&topic).and(Ok(topic)))
    }

    pub fn create_chat(&self, user_id: String) -> Result<Topic> {
        self.json_call(
            &format!("/api/topic/create/{}", user_id),
            serde_json::json!({}),
            None,
        )
        .and_then(|topic: Topic| self.db.save_topic(&topic).and(Ok(topic)))
    }

    pub fn update_topic_notice(&self, topic_id: String, text: String) -> Result<()> {
        let data = serde_json::json!({
            "topicId": topic_id,
            "text": text
        });

        self.json_call::<CommonResp>(&format!("/api/topic/admin/notice/{}", topic_id), data, None)
            .and({
                let user_id = self.db.get_value(super::KEY_USER_ID)?;
                let notice = Some(TopicNotice::new(
                    &text,
                    &user_id,
                    &chrono::Utc::now().to_rfc3339(),
                ));
                self.db.update_topic_notice(&topic_id, notice)
            })
            .and(Ok(()))
    }

    pub fn silent_topic(&self, topic_id: String, duration: String) -> Result<()> {
        let data = serde_json::json!({ "duration": duration });
        self.json_call::<CommonResp>(
            &format!("/api/topic/admin/silent_topic/{}", topic_id),
            data,
            None,
        )
        .and(self.db.silent_topic(&topic_id, true))
        .and(Ok(()))
    }

    pub fn silent_topic_member(
        &self,
        topic_id: String,
        user_id: String,
        duration: String,
    ) -> Result<()> {
        let data = serde_json::json!({ "duration": duration });
        self.json_call::<CommonResp>(
            &format!("/api/topic/admin/silent/{}/{}", topic_id, user_id),
            data,
            None,
        )
        .and(self.db.silent_topic_member(&topic_id, &user_id, true))
        .and(Ok(()))
    }

    pub fn quit_topic(&self, topic_id: String) -> Result<()> {
        self.json_call::<CommonResp>(
            &format!("/api/topic/quit/{}", topic_id),
            serde_json::json!({}),
            None,
        )
        .and({
            let user_id = self.db.get_value(super::KEY_USER_ID)?;
            self.db.remove_topic_member(&topic_id, &user_id)
        })
        .and(Ok(()))
    }

    pub fn dismiss_topic(&self, topic_id: String) -> Result<()> {
        self.json_call::<CommonResp>(
            &format!("/api/topic/dismiss/{}", topic_id),
            serde_json::json!({}),
            None,
        )
        .and(self.db.dismiss_topic(&topic_id))
        .and(Ok(()))
    }

    pub fn join_topic(&self, topic_id: String, message: String, source: String) -> Result<()> {
        let data = serde_json::json!({
            "message": message,
            "source": source,
        });
        self.json_call::<CommonResp>(&format!("/api/topic/knock/{}", topic_id), data, None)
            .and(Ok(()))
    }

    pub fn get_topic_knocks(&self, topic_id: String) -> Result<Vec<TopicKnock>> {
        let topic_knocks: Vec<TopicKnock> = self.json_call(
            &format!("/api/topic/admin/list_knock/{}", topic_id),
            serde_json::json!({}),
            None,
        )?;
        let mut topics_knock_count = self
            .db
            .get_value(super::KEY_TOPICS_KNOCK_COUNT)
            .unwrap_or("0".to_string())
            .parse::<u32>()
            .unwrap_or(0);
        topics_knock_count = topics_knock_count + topic_knocks.len() as u32;
        self.db.set_value(
            super::KEY_TOPICS_KNOCK_COUNT,
            topics_knock_count.to_string().as_str(),
        )?;
        Ok(topic_knocks)
    }

    pub fn accept_topic_join(&self, topic_id: String, user_id: String, memo: String) -> Result<()> {
        let data = serde_json::json!({ "memo": memo });
        self.json_call::<CommonResp>(
            &format!("/api/topic/admin/knock/accept/{}/{}", topic_id, user_id),
            data,
            None,
        )
        .and(Ok(()))
    }

    pub fn decline_topic_join(
        &self,
        topic_id: String,
        user_id: String,
        message: String,
    ) -> Result<()> {
        let data = serde_json::json!({
            "message": message,
        });
        self.json_call::<CommonResp>(
            &format!("/api/topic/admin/knock/reject/{}/{}", topic_id, user_id),
            data,
            None,
        )
        .and(Ok(()))
    }

    pub fn remove_topic_member(&self, topic_id: String, user_id: String) -> Result<()> {
        self.json_call::<CommonResp>(
            &format!("/api/topic/admin/kickout/{}/{}", topic_id, user_id),
            serde_json::json!({}),
            None,
        )
        .and(self.db.remove_topic_member(&topic_id, &user_id))
        .and(Ok(()))
    }

    pub fn sync_chatlogs(&self, topic_id: String, start_seq: u64, end_seq: u64) -> Result<()> {
        self.ctrl_tx
            .send(super::CtrlMessageType::ChatLogSync(
                topic_id, start_seq, end_seq,
            ))
            .map_err(|e| crate::error::ClientError::SendCtrlMessageError(e.to_string()))
    }

    pub fn begin_sync_chatlogs(
        &self,
        topic_id: String,
        start_seq: u64,
        end_seq: u64,
    ) -> Result<()> {
        loop {
            let lr = self.get_chat_logs_desc(topic_id.clone(), start_seq, end_seq)?;
            let has_more = lr.has_more;
            let chat_logs_sync_at = lr.updated_at.clone();
            debug!(
                "sync logs start_seq:{} end_seq:{} counts:{} last_seq:{}",
                start_seq,
                end_seq,
                lr.items.len(),
                lr.last_seq,
            );

            if let Some(cb) = self.callback.read().unwrap().as_ref() {
                cb.on_topic_logs_sync(topic_id.clone(), lr);
            }

            if !has_more {
                self.db.set_value(&topic_id, &chat_logs_sync_at)?;
                break;
            }
        }
        Ok(())
    }

    // 倒序的获取聊天记录
    pub fn get_chat_logs_desc(
        &self,
        topic_id: String,
        start_seq: u64,
        end_seq: u64,
    ) -> Result<ListChatLogResult> {
        let data = serde_json::json!({
            "topicId": topic_id,
            "lastSeq": start_seq,
            "maxSeq": end_seq,
            "limit": LIMIT
        });

        let mut lr: ListChatLogResult =
            self.json_call(&format!("/api/chat/sync/{}", topic_id), data, None)?;

        lr.items.iter_mut().try_for_each(|c| -> Result<()> {
            c.cached_at = chrono::Utc::now().to_rfc3339();
            self.db.save_chat_log(&c).ok();
            Ok(())
        })?;

        let last_seq = lr.items.iter().map(|c| c.seq).max().unwrap_or(0);

        debug!(
            "get logs start_seq:{} end_seq:{} counts:{} last_seq:{}",
            start_seq,
            end_seq,
            lr.items.len(),
            last_seq
        );
        return Ok(lr);
    }

    pub fn get_chat_log(&self, topic_id: String, id: String) -> Result<ChatLog> {
        self.db.get_chat_log(&topic_id, &id)
    }

    pub fn search_chat_log(
        &self,
        topic_id: String,
        sender_id: String,
        keyword: String,
    ) -> Result<Vec<ChatLog>> {
        self.db.search_chat_log(&topic_id, &sender_id, &keyword)
    }

    pub fn get_topics_knock_count(&self) -> Result<u32> {
        Ok(self
            .db
            .get_value(super::KEY_TOPICS_KNOCK_COUNT)?
            .parse::<u32>()?)
    }

    pub fn get_conversations_count(&self) -> Result<u32> {
        self.db.get_conversations_count()
    }
}

#[allow(unused_variables)]
impl Client {
    pub fn sync_conversations(&self, without_cache: bool) -> Result<()> {
        self.ctrl_tx
            .send(super::CtrlMessageType::ConversationSync(without_cache))
            .map_err(|e| crate::error::ClientError::SendCtrlMessageError(e.to_string()))
    }

    pub(crate) fn begin_sync_conversations(&self, without_cache: bool) -> Result<()> {
        let mut conversations_sync_at = match without_cache {
            true => String::default(),
            false => match self.db.get_value(KEY_CONVERSATIONS_SYNC_AT) {
                Ok(v) => {
                    if is_expired(&v, CONVERSATIONS_SYNC) {
                        String::default()
                    } else {
                        v
                    }
                }
                Err(_) => String::default(),
            },
        };

        loop {
            let lr = self.get_conversations(conversations_sync_at, LIMIT)?;
            if let Some(cb) = self.callback.read().unwrap().as_ref() {
                cb.on_conversation_updated(lr.items)
            }
            conversations_sync_at = lr.updated_at;
            if !lr.has_more {
                break;
            }
        }
        self.db
            .set_value(KEY_CONVERSATIONS_SYNC_AT, &conversations_sync_at)?;
        Ok(())
    }

    pub fn set_conversation_read(&self, topic_id: String) -> Result<()> {
        self.db.set_conversation_read(&topic_id)?;
        let r: CommonResp = self.json_call(
            &format!("/api/chat/read/{}", topic_id),
            serde_json::json!({}),
            None,
        )?;
        Ok(())
    }

    pub fn update_topic(&self, topic_id: String, name: String, icon: String) -> Result<()> {
        let vals = serde_json::json!({
            "name":name,
            "icon":icon,
        });
        let r: CommonResp = self.json_call(
            &format!("/api/topic/admin/add_admin/{}", topic_id),
            vals,
            None,
        )?;
        Ok(())
    }

    pub fn add_topic_admin(&self, topic_id: String, user_id: String) -> Result<()> {
        let r: CommonResp = self.json_call(
            &format!("/api/topic/admin/add_admin/{}/{}", topic_id, user_id),
            serde_json::json!({}),
            None,
        )?;
        Ok(())
    }

    pub fn remove_topic_admin(&self, topic_id: String, user_id: String) -> Result<()> {
        let r: CommonResp = self.json_call(
            &format!("/api/topic/admin/remove_admin/{}/{}", topic_id, user_id),
            serde_json::json!({}),
            None,
        )?;
        Ok(())
    }

    pub fn transfer_topic(&self, topic_id: String, user_id: String) -> Result<()> {
        let r: CommonResp = self.json_call(
            &format!("/api/topic/admin/transfer/{}/{}", topic_id, user_id),
            serde_json::json!({}),
            None,
        )?;
        Ok(())
    }

    pub fn invite_topic_member(&self, topic_id: String, user_id: String) -> Result<()> {
        let r: CommonResp = self.json_call(
            &format!("/api/topic/invite/{}/{}", topic_id, user_id),
            serde_json::json!({}),
            None,
        )?;
        Ok(())
    }

    pub fn clean_topic_history(&self, topic_id: String, sync: bool) -> Result<()> {
        let vals = serde_json::json!({});
        let r: CommonResp = self.json_call(
            &format!("/api/chat/clear_messages/{}", topic_id),
            vals,
            None,
        )?;
        Ok(())
    }

    pub fn remove_messages(
        &self,
        topic_id: String,
        chat_ids: Vec<String>,
        sync: bool,
    ) -> Result<()> {
        let vals = serde_json::json!({ "ids": chat_ids });
        let r: CommonResp = self.json_call(
            &format!("/api/chat/remove_messages/{}", topic_id),
            vals,
            None,
        )?;
        Ok(())
    }

    pub fn set_user_remark(&self, user_id: String, remark: String) -> Result<()> {
        self.db.set_user_remark(&user_id, &remark)?;
        let vals = serde_json::json!({ "remark": remark });
        let r: CommonResp = self.json_call(&format!("/api/relation/{}", user_id), vals, None)?;
        Ok(())
    }

    pub fn set_user_star(&self, user_id: String, star: bool) -> Result<()> {
        self.db.set_user_star(&user_id, star)?;
        let vals = serde_json::json!({ "favorite": star });
        let r: CommonResp = self.json_call(&format!("/api/relation/{}", user_id), vals, None)?;
        Ok(())
    }

    pub fn set_allow_guest_chat(&self, allowed: bool) -> Result<()> {
        let vals = serde_json::json!({ "allowGuest": allowed });
        let r: CommonResp = self.json_call(&format!("/api/profile/update"), vals, None)?;
        Ok(())
    }
}
