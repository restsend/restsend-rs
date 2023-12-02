use std::io::Write;
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

pub fn init_log(level: &str, is_test: bool) {
    let _ = env_logger::builder()
        .is_test(is_test)
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}] {}:{} - {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                record.args()
            )
        })
        .format_timestamp(None)
        .filter_level(level.parse().unwrap())
        .try_init();
}
#[cfg(test)]
pub(crate) async fn check_until(
    duration: std::time::Duration,
    f: impl Fn() -> bool,
) -> anyhow::Result<()> {
    let st = std::time::Instant::now();
    loop {
        if f() {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        if st.elapsed() > duration {
            return Err(anyhow::anyhow!("check_until timeout: {:?}", duration));
        }
    }
}
