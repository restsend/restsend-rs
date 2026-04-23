use reqwest::Client;
use serde::Serialize;
use std::time::Duration;

#[derive(Clone)]
pub struct WebhookSender {
    client: Client,
    retries: usize,
}

impl WebhookSender {
    pub fn new(timeout_secs: u64, retries: usize) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs.max(1)))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            client,
            retries: retries.max(1),
        }
    }

    pub async fn send_json<T: Serialize + ?Sized>(
        &self,
        url: &str,
        payload: &T,
    ) -> Result<(), reqwest::Error> {
        let mut attempt = 0usize;
        loop {
            let resp = self.client.post(url).json(payload).send().await;
            match resp {
                Ok(resp) => match resp.error_for_status() {
                    Ok(_) => return Ok(()),
                    Err(err) => {
                        attempt += 1;
                        if attempt >= self.retries {
                            return Err(err);
                        }
                    }
                },
                Err(err) => {
                    attempt += 1;
                    if attempt >= self.retries {
                        return Err(err);
                    }
                }
            }
            tokio::time::sleep(Duration::from_secs(attempt as u64)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::WebhookSender;
    use axum::http::StatusCode;
    use axum::routing::post;
    use axum::Router;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn send_json_retries_until_success() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let app = Router::new().route(
            "/hook",
            post({
                let attempts = attempts.clone();
                move || {
                    let attempts = attempts.clone();
                    async move {
                        let attempt = attempts.fetch_add(1, Ordering::SeqCst) + 1;
                        if attempt == 1 {
                            StatusCode::INTERNAL_SERVER_ERROR
                        } else {
                            StatusCode::OK
                        }
                    }
                }
            }),
        );

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let sender = WebhookSender::new(2, 2);
        let url = format!("http://{addr}/hook");
        let payload = serde_json::json!({"name": "chat"});

        let result = sender.send_json(&url, &payload).await;
        assert!(result.is_ok(), "expected retry to eventually succeed");
        assert_eq!(attempts.load(Ordering::SeqCst), 2);

        server.abort();
    }

    #[tokio::test]
    async fn send_json_returns_error_after_max_retries() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let app = Router::new().route(
            "/hook",
            post({
                let attempts = attempts.clone();
                move || {
                    let attempts = attempts.clone();
                    async move {
                        attempts.fetch_add(1, Ordering::SeqCst);
                        StatusCode::INTERNAL_SERVER_ERROR
                    }
                }
            }),
        );

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let sender = WebhookSender::new(2, 2);
        let url = format!("http://{addr}/hook");
        let payload = serde_json::json!({"name": "chat"});

        let result = sender.send_json(&url, &payload).await;
        assert!(result.is_err(), "expected retries exhausted error");
        assert_eq!(attempts.load(Ordering::SeqCst), 2);

        server.abort();
    }
}
