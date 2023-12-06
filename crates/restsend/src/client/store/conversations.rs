use super::ClientStore;
use crate::{
    client::store::StoreEvent,
    models::{ChatLog, ChatLogStatus, Content, ContentType, Conversation},
    request::ChatRequest,
    services::conversation::get_conversation,
    storage::{QueryOption, QueryResult},
    utils::now_timestamp,
    MAX_RECALL_SECS,
};
use anyhow::Result;
use log::warn;

impl ClientStore {
    pub(super) async fn fetch_conversation(&self, topic_id: &str) {
        if let Some(event_tx) = self.event_tx.lock().unwrap().clone() {
            let endpoint = self.endpoint.clone();
            let token = self.token.clone();
            let topic_id = topic_id.to_string();

            tokio::spawn(async move {
                let converstion = get_conversation(&endpoint, &token, &topic_id).await;
                if let Ok(converstion) = converstion {
                    event_tx
                        .send(StoreEvent::UpdateConversation(vec![converstion]))
                        .ok();
                } else {
                    warn!("fetch_conversation failed");
                }
            });
        }
    }

    pub(super) fn update_conversation(
        &self,
        mut conversation: Conversation,
    ) -> Result<Conversation> {
        let t = self
            .message_storage
            .table::<Conversation>("conversations")
            .ok_or(anyhow::anyhow!(
                "update_conversation_from_chat: get table failed"
            ))?;

        let topic_id = conversation.topic_id.clone();

        if let Some(old_conversation) = t.get("", &topic_id) {
            if old_conversation.last_seq <= conversation.last_seq {
                conversation.last_read_seq = old_conversation.last_read_seq;
                conversation.last_sender_id = old_conversation.last_sender_id.clone();
                conversation.last_message_at = old_conversation.last_message_at.clone();
                conversation.last_message = old_conversation.last_message.clone();

                // TODO: update other fields
                conversation.multiple = old_conversation.multiple;
                conversation.mute = old_conversation.mute;
                conversation.sticky = old_conversation.sticky;
            }
        }

        conversation.is_partial = false;
        conversation.cached_at = now_timestamp();
        conversation.unread = (conversation.last_seq - conversation.last_read_seq).max(0);

        t.set("", &topic_id, Some(conversation.clone()));
        Ok(conversation)
    }

    pub(super) fn update_conversation_from_chat(&self, req: &ChatRequest) -> Result<Conversation> {
        let topic_id = &req.topic_id;
        let t = self
            .message_storage
            .table::<Conversation>("conversations")
            .ok_or(anyhow::anyhow!(
                "update_conversation_from_chat: get table failed"
            ))?;

        let mut conversation = t.get("", &topic_id).unwrap_or(Conversation::from(req));

        if req.seq >= conversation.last_seq {
            conversation.last_seq = req.seq;
            conversation.last_sender_id = req.attendee.clone();
            conversation.last_message_at = req.created_at.clone();
            conversation.last_message = req.content.clone();
            conversation.cached_at = now_timestamp();
            conversation.unread = (conversation.last_seq - conversation.last_read_seq).max(0);
        }

        Ok(conversation)
    }

    pub(super) fn update_conversation_read(&self, topic_id: &str) -> Result<()> {
        let t = self
            .message_storage
            .table::<Conversation>("conversations")
            .ok_or(anyhow::anyhow!(
                "update_conversation_from_chat: get table failed"
            ))?;

        if let Some(mut conversation) = t.get("", topic_id) {
            conversation.last_read_seq = conversation.last_seq;
            t.set("", topic_id, Some(conversation));
        }
        Ok(())
    }

    pub(super) fn save_outgoing_chat_log(&self, req: &ChatRequest) -> Result<()> {
        let t = self
            .message_storage
            .table::<ChatLog>("chat_logs")
            .ok_or(anyhow::anyhow!("save_outgoing_chat_log: get table failed"))?;

        let mut log = ChatLog::from(req);
        log.status = ChatLogStatus::Sending;
        log.sender_id = self.user_id.clone();
        t.set(&log.topic_id, &log.id, Some(log.clone()));

        Ok(())
    }

    pub(super) fn update_outoing_chat_log_state(
        &self,
        topic_id: &str,
        chat_id: &str,
        status: ChatLogStatus,
        seq: Option<i64>,
    ) -> Result<()> {
        let t = self
            .message_storage
            .table::<ChatLog>("chat_logs")
            .ok_or(anyhow::anyhow!("save_outgoing_chat_log: get table failed"))?;

        warn!(
            "update_outoing_chat_log_state: topic_id: {} chat_id: {}, status: {:?} seq: {:?}",
            topic_id, chat_id, status, seq
        );

        if let Some(log) = t.get(topic_id, chat_id) {
            let mut log = log.clone();
            log.status = status;
            if let Some(seq) = seq {
                log.seq = seq;
            }
            t.set(topic_id, chat_id, Some(log));
        }
        Ok(())
    }

    pub(super) fn save_incoming_chat_log(&self, req: &ChatRequest) -> Result<()> {
        let t = self
            .message_storage
            .table::<ChatLog>("chat_logs")
            .ok_or(anyhow::anyhow!("save_incoming_chat_log: get table failed"))?;
        let topic_id = &req.topic_id;
        let chat_id = &req.chat_id;
        let now = now_timestamp();

        if let Some(old_log) = t.get(&topic_id, &chat_id) {
            if req.r#type == "recall" {
                if now - old_log.cached_at > MAX_RECALL_SECS {
                    return Err(anyhow::anyhow!("[recall] timeout"));
                }

                match old_log.status {
                    ChatLogStatus::Received => {}
                    _ => return Err(anyhow::anyhow!("[recall] invalid status")),
                }

                if req.attendee != old_log.sender_id {
                    return Err(anyhow::anyhow!("[recall] invalid  owner"));
                }

                let mut log = old_log.clone();
                log.recall = true;
                log.content = Content::new(ContentType::Recall);
                t.set(&topic_id, &chat_id, Some(log));
            }
            return Ok(());
        }

        let mut log = ChatLog::from(req);
        log.status = ChatLogStatus::Received;
        log.cached_at = now;

        // TODO: download attachment
        t.set(&log.topic_id, &log.id, Some(log.clone()));
        Ok(())
    }

    pub(crate) fn save_chat_log(&self, chat_log: &ChatLog) -> Result<()> {
        let t = self
            .message_storage
            .table("chat_logs")
            .ok_or(anyhow::anyhow!("save_chat_log: get table failed"))?;

        if let Some(_) = t.get(&chat_log.topic_id, &chat_log.id) {
            return Ok(());
        }

        let item = match ContentType::from(chat_log.content.r#type.to_string()) {
            ContentType::None => None, // remove local log
            _ => Some(chat_log.clone()),
        };

        t.set(&chat_log.topic_id, &chat_log.id, item);
        Ok(())
    }

    pub async fn get_chat_logs(
        &self,
        topic_id: &str,
        seq: i64,
        limit: u32,
    ) -> Result<QueryResult<ChatLog>> {
        let t = self
            .message_storage
            .table("chat_logs")
            .ok_or(anyhow::anyhow!("get_chat_logs: get table failed"))?;

        let start_sort_value = (seq - limit as i64).max(0);
        let option = QueryOption {
            keyword: None,
            start_sort_value,
            limit,
        };
        Ok(t.query(topic_id, &option))
    }
}
