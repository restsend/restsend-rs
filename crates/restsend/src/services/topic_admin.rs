use anyhow::Result;
pub async fn update_topic_notice(
    endpoint: &str,
    token: &str,
    topic_id: String,
    text: String,
) -> Result<()> {
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

pub async fn silent_topic(
    endpoint: &str,
    token: &str,
    topic_id: String,
    duration: String,
) -> Result<()> {
    let data = serde_json::json!({ "duration": duration });
    self.json_call::<CommonResp>(
        &format!("/api/topic/admin/silent_topic/{}", topic_id),
        data,
        None,
    )
    .and(self.db.silent_topic(&topic_id, true))
    .and(Ok(()))
}

pub async fn silent_topic_member(
    endpoint: &str,
    token: &str,
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

pub async fn quit_topic(endpoint: &str, token: &str, topic_id: String) -> Result<()> {
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

pub async fn dismiss_topic(endpoint: &str, token: &str, topic_id: String) -> Result<()> {
    self.json_call::<CommonResp>(
        &format!("/api/topic/dismiss/{}", topic_id),
        serde_json::json!({}),
        None,
    )
    .and(self.db.dismiss_topic(&topic_id))
    .and(Ok(()))
}

pub async fn get_topic_knocks(
    endpoint: &str,
    token: &str,
    topic_id: String,
) -> Result<Vec<TopicKnock>> {
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

pub async fn accept_topic_join(
    endpoint: &str,
    token: &str,
    topic_id: String,
    user_id: String,
    memo: String,
) -> Result<()> {
    let data = serde_json::json!({ "memo": memo });
    self.json_call::<CommonResp>(
        &format!("/api/topic/admin/knock/accept/{}/{}", topic_id, user_id),
        data,
        None,
    )
    .and(Ok(()))
}

pub async fn decline_topic_join(
    endpoint: &str,
    token: &str,
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

pub async fn remove_topic_member(
    endpoint: &str,
    token: &str,
    topic_id: String,
    user_id: String,
) -> Result<()> {
    self.json_call::<CommonResp>(
        &format!("/api/topic/admin/kickout/{}/{}", topic_id, user_id),
        serde_json::json!({}),
        None,
    )
    .and(self.db.remove_topic_member(&topic_id, &user_id))
    .and(Ok(()))
}

pub async fn update_topic(
    endpoint: &str,
    token: &str,
    topic_id: String,
    name: String,
    icon: String,
) -> Result<()> {
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

pub async fn add_topic_admin(
    endpoint: &str,
    token: &str,
    topic_id: String,
    user_id: String,
) -> Result<()> {
    let r: CommonResp = self.json_call(
        &format!("/api/topic/admin/add_admin/{}/{}", topic_id, user_id),
        serde_json::json!({}),
        None,
    )?;
    Ok(())
}

pub async fn remove_topic_admin(
    endpoint: &str,
    token: &str,
    topic_id: String,
    user_id: String,
) -> Result<()> {
    let r: CommonResp = self.json_call(
        &format!("/api/topic/admin/remove_admin/{}/{}", topic_id, user_id),
        serde_json::json!({}),
        None,
    )?;
    Ok(())
}

pub async fn transfer_topic(
    endpoint: &str,
    token: &str,
    topic_id: String,
    user_id: String,
) -> Result<()> {
    let r: CommonResp = self.json_call(
        &format!("/api/topic/admin/transfer/{}/{}", topic_id, user_id),
        serde_json::json!({}),
        None,
    )?;
    Ok(())
}
