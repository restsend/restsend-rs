use std::{io::Write, time::Duration};
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

#[uniffi::export]
pub fn now_millis() -> i64 {
    chrono::Local::now().timestamp_millis()
}

pub fn now_secs() -> i64 {
    chrono::Local::now().timestamp()
}

pub fn elapsed(d: i64) -> Duration {
    Duration::from_millis((now_millis() - d).abs() as u64)
}

#[uniffi::export]
pub fn init_log(level: String, is_test: bool) {
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
) -> crate::Result<()> {
    use crate::error::ClientError;

    let st = crate::utils::now_millis();
    loop {
        if f() {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        if crate::utils::elapsed(st) > duration {
            return Err(ClientError::Other(format!(
                "check_until timeout: {:?}",
                duration
            )));
        }
    }
}
