use anyhow::{anyhow, Result};
use std::time::Duration;
use tokio::time::{sleep, Instant};

mod test_client;

const TEST_ENDPOINT: &str = "https://chat.ruzhila.cn";

async fn check_until(duration: Duration, f: impl Fn() -> bool) -> Result<()> {
    let st = Instant::now();
    loop {
        if f() {
            return Ok(());
        }
        sleep(Duration::from_millis(100)).await;
        if st.elapsed() > duration {
            return Err(anyhow!("check_until timeout: {:?}", duration));
        }
    }
}
