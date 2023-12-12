use std::{io::Write, time::Duration};
#[cfg(not(target_arch = "wasm32"))]
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

#[cfg(target_arch = "wasm32")]
// a hex random text with length of count, lowercase
pub fn random_text(count: usize) -> String {
    use js_sys::Math;
    let mut s = String::new();
    for _ in 0..count {
        let r = Math::random();
        let c = (r * 16.0) as u8;
        let c = if c < 10 {
            (c + 48) as char
        } else {
            (c + 87) as char
        };
        s.push(c);
    }
    s
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

#[cfg(not(target_arch = "wasm32"))]
pub async fn sleep(duration: Duration) {
    tokio::time::sleep(duration).await
}

#[cfg(target_arch = "wasm32")]
pub async fn sleep(duration: Duration) {
    use wasm_bindgen::prelude::*;
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_name = setTimeout)]
        pub fn set_timeout(closure: &wasm_bindgen::closure::Closure<dyn FnMut()>, time: u32);
    }
    let p = js_sys::Promise::new(&mut |resolve, _| {
        let closure = Closure::new(move || {
            let this = JsValue::null();
            let _ = resolve.call0(&this);
        });
        set_timeout(&closure, d.as_millis() as u32);
        closure.forget();
    });
    wasm_bindgen_futures::JsFuture::from(p).await.ok();
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
        sleep(std::time::Duration::from_millis(100)).await;
        if crate::utils::elapsed(st) > duration {
            return Err(ClientError::Other(format!(
                "check_until timeout: {:?}",
                duration
            )));
        }
    }
}
