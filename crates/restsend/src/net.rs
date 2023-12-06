use crate::utils::random_text;
use crate::{ClientError, NetworkState, Result};
use reqwest::header::{
    HeaderMap, AUTHORIZATION, CONNECTION, CONTENT_TYPE, HOST, SEC_WEBSOCKET_KEY,
    SEC_WEBSOCKET_VERSION, UPGRADE,
};
use reqwest::Url;
use reqwest::{Body, ClientBuilder, RequestBuilder};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;
use tokio::time::Duration;
pub struct ConnectionState {
    is_active: AtomicBool,
    broken_count: AtomicUsize,
    connected_at: AtomicU64,
    broken_at: AtomicU64,
}

impl ConnectionState {
    pub fn new() -> Self {
        ConnectionState {
            is_active: AtomicBool::new(false),
            broken_count: AtomicUsize::new(0),
            connected_at: AtomicU64::new(0),
            broken_at: AtomicU64::new(0),
        }
    }

    pub fn did_connect(&self) {
        self.connected_at.store(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            Ordering::Relaxed,
        );
        self.broken_count.store(0, Ordering::Relaxed);
    }

    pub fn did_broken(&self) {
        self.broken_count.fetch_add(1, Ordering::Relaxed);
        self.broken_at.store(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            Ordering::Relaxed,
        );
    }

    pub fn is_active(&self) -> bool {
        self.is_active.load(Ordering::Relaxed)
    }

    pub fn set_active(&self, val: bool) {
        self.is_active.store(val, Ordering::Relaxed);
    }

    pub fn is_timeout(&self) -> Result<bool> {
        if self.broken_count.load(Ordering::Relaxed) == 0 {
            return Ok(false);
        }

        let broken_at = self.broken_at.load(Ordering::Relaxed);
        if broken_at == 0 {
            return Ok(false);
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();

        let wait_secs = now - broken_at;
        if wait_secs > self.wait_connect_secs()? as u64 {
            return Ok(false);
        }
        Ok(true)
    }

    pub fn wait_connect_secs(&self) -> Result<usize> {
        let count = {
            let c = self.broken_count.load(Ordering::Relaxed);
            if c <= 0 {
                return Ok(0);
            } else if c >= 5 {
                // 如果断开次数大于5次，那么最多等待20秒
                return Ok(20);
            }
            c
        };
        // 如果是活跃状态，那么最多等待3秒
        if self.is_active() && count >= 3 {
            return Ok(3);
        }
        Ok(count)
    }
}

#[derive(Default)]
pub struct NetStore {
    me: Mutex<Option<String>>,
    endpoint: Mutex<String>,
    state: Mutex<NetworkState>,
    auth_token: Mutex<String>,
    is_running: AtomicBool,
}

impl NetStore {
    pub fn new(endpoint: String) -> Self {
        NetStore {
            endpoint: Mutex::new(endpoint),
            ..Default::default()
        }
    }

    pub fn me(&self) -> Result<String> {
        let r = self.me.lock().unwrap();
        match r.as_ref() {
            Some(s) => Ok(s.clone()),
            None => Err(ClientError::NetworkBroken(
                "NetStore::me is empty".to_string(),
            )),
        }
    }

    pub fn set_me(&self, me: String) {
        *self.me.lock().unwrap() = Some(me);
    }

    pub fn endpoint(&self) -> Result<String> {
        Ok(self.endpoint.lock()?.clone())
    }

    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    pub fn set_running(&self, val: bool) {
        self.is_running.store(val, Ordering::Relaxed);
    }

    pub fn set_auth_token(&self, token: &String) {
        *self.auth_token.lock().unwrap() = token.clone();
    }

    pub fn auth_token(&self) -> String {
        self.auth_token.lock().unwrap().clone()
    }

    pub fn get_state(&self) -> NetworkState {
        self.state.lock().unwrap().clone()
    }

    pub fn set_state(&self, val: NetworkState) {
        *self.state.lock().unwrap() = val;
    }

    pub fn build_http_headers(&self, exists_headers: Option<HeaderMap>) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        if let Some(h) = exists_headers {
            for (k, v) in h {
                headers.append(k.unwrap(), v);
            }
        }

        // check Content-Type is exists in headers or put application/json as default
        if !headers.contains_key(CONTENT_TYPE) {
            headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
        }
        let token = self.auth_token();
        if !token.is_empty() {
            // format with Bearer token
            headers.insert(AUTHORIZATION, format!("Bearer {}", token).parse().unwrap());
        }
        Ok(headers)
    }

    pub fn build_headers_for_websocket(&self, url: &String) -> Result<HeaderMap> {
        let mut headers = self.build_http_headers(None)?;

        let host = Url::parse(&url)?.host_str().unwrap().to_string();
        headers.insert(HOST, host.parse().unwrap());
        headers.insert(CONNECTION, "Upgrade".parse().unwrap());
        headers.insert(UPGRADE, "websocket".parse().unwrap());
        headers.insert(SEC_WEBSOCKET_VERSION, "13".parse().unwrap());
        headers.insert(SEC_WEBSOCKET_KEY, random_text(16).parse().unwrap());

        Ok(headers)
    }

    pub fn make_request<T: Into<Body>>(
        &self,
        method: http::Method,
        url: &String,
        headers: Option<HeaderMap>,
        body: T,
        timeout_secs: u64,
    ) -> Result<RequestBuilder> {
        let req = ClientBuilder::new().user_agent(crate::USER_AGENT).build()?;
        let builder = match method {
            http::Method::GET => req.get(url),
            http::Method::POST => req.post(url).body(body),
            _ => {
                return Err(ClientError::HTTPError(format!(
                    "unsupported http method: {}",
                    method
                )));
            }
        }
        .headers(self.build_http_headers(headers)?)
        .timeout(Duration::from_secs(timeout_secs));
        Ok(builder)
    }

    pub fn make_post_request(
        &self,
        url: &String,
        headers: Option<HeaderMap>,
        timeout_secs: u64,
    ) -> Result<RequestBuilder> {
        let req = ClientBuilder::new().user_agent(crate::USER_AGENT).build()?;
        let token = self.auth_token();

        let mut headers = headers.unwrap_or_default();
        if !token.is_empty() {
            headers.insert(AUTHORIZATION, format!("Bearer {}", token).parse().unwrap());
        }

        let builder = req
            .post(url)
            .headers(headers)
            .timeout(Duration::from_secs(timeout_secs));
        Ok(builder)
    }
}
