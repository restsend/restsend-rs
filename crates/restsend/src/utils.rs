// a hex random text with length of count, lowercase
pub fn random_text(count: usize) -> String {
    use rand::Rng;
    rand::thread_rng()
        .sample_iter(rand::distributions::Alphanumeric)
        .take(count)
        .map(char::from)
        .collect::<String>()
        .to_lowercase()
}

pub fn is_expired(time: &str, seconds: i64) -> bool {
    if let Ok(_time) = chrono::DateTime::parse_from_rfc3339(time) {
        if chrono::Utc::now()
            .signed_duration_since(_time)
            .gt(&chrono::Duration::seconds(seconds))
        {
            return true;
        }
    }
    false
}
