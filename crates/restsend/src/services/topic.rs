use anyhow::Result;

pub async fn get_topic(endpoint: &str, token: &str, topic_id: String) -> Result<Topic> {
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

pub async fn get_topic_admins(endpoint: &str, token: &str, topic_id: String) -> Result<Vec<User>> {
    let admins = self.db.get_topic_admins(&topic_id)?;
    if admins.is_empty() {
        return Ok(vec![]);
    }
    let data = serde_json::json!({ "ids": admins });
    self.json_call("/api/profile", data, None)
}

pub async fn get_topic_members(
    endpoint: &str,
    token: &str,
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

pub async fn get_topic_owner(endpoint: &str, token: &str, topic_id: String) -> Result<User> {
    let owner_id = self.db.get_topic_owner(&topic_id)?;
    self.get_user(owner_id)
}

pub async fn create_topic(
    endpoint: &str,
    token: &str,
    name: String,
    icon: String,
    members: Vec<String>,
) -> Result<Topic> {
    let data = serde_json::json!({
        "name": name,
        "icon": icon,
        "members": members
    });
    self.json_call("/api/topic/create", data, None)
        .and_then(|topic: Topic| self.db.save_topic(&topic).and(Ok(topic)))
}

pub async fn create_chat(endpoint: &str, token: &str, user_id: String) -> Result<Topic> {
    self.json_call(
        &format!("/api/topic/create/{}", user_id),
        serde_json::json!({}),
        None,
    )
    .and_then(|topic: Topic| self.db.save_topic(&topic).and(Ok(topic)))
}

pub async fn join_topic(
    endpoint: &str,
    token: &str,
    topic_id: String,
    message: String,
    source: String,
) -> Result<()> {
    let data = serde_json::json!({
        "message": message,
        "source": source,
    });
    self.json_call::<CommonResp>(&format!("/api/topic/knock/{}", topic_id), data, None)
        .and(Ok(()))
}

pub async fn invite_topic_member(
    endpoint: &str,
    token: &str,
    topic_id: String,
    user_id: String,
) -> Result<()> {
    let r: CommonResp = self.json_call(
        &format!("/api/topic/invite/{}/{}", topic_id, user_id),
        serde_json::json!({}),
        None,
    )?;
    Ok(())
}
