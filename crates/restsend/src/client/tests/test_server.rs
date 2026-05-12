#![cfg(not(target_arch = "wasm32"))]

use restsend_backend::app::{build_router, AppConfig};
use tokio::net::TcpListener;

pub(crate) struct LocalTestServer {
    pub(crate) endpoint: String,
    server: tokio::task::JoinHandle<()>,
}

impl LocalTestServer {
    pub(crate) async fn start() -> Self {
        let config = AppConfig {
            addr: "127.0.0.1:0".to_string(),
            endpoint: "127.0.0.1:0".to_string(),
            database_url: format!(
                "sqlite:file:restsend-sdk-e2e-{}?mode=memory&cache=shared",
                crate::utils::random_text(8)
            ),
            openapi_schema: "http".to_string(),
            openapi_prefix: "/open".to_string(),
            api_prefix: "/api".to_string(),
            log_file: format!("logs/sdk-e2e-{}.log", crate::utils::random_text(8)),
            openapi_token: Some("test-token".to_string()),
            run_migrations: true,
            migrate_only: false,
            webhook_timeout_secs: 5,
            webhook_retries: 2,
            webhook_targets: vec![],
            event_bus_size: 256,
            message_worker_count: 2,
            message_queue_size: 64,
            push_worker_count: 2,
            push_queue_size: 64,
            webhook_worker_count: 2,
            webhook_queue_size: 64,
            max_upload_bytes: 10 * 1024 * 1024,
            presence_backend: "memory".to_string(),
            presence_node_id: "sdk-e2e-node".to_string(),
            presence_ttl_secs: 90,
            presence_heartbeat_secs: 10,
            ws_per_user_limit: 0,
            ws_client_queue_size: 0,
            ws_typing_interval_ms: 1000,
            ws_drop_on_backpressure: true,
        };

        let (app, state) = build_router(config).await.expect("build router");
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let endpoint = format!("http://{}", addr);
        let server = tokio::spawn(async move {
            axum::serve(listener, app.with_state(state)).await.unwrap();
        });

        Self { endpoint, server }
    }
}

impl Drop for LocalTestServer {
    fn drop(&mut self) {
        self.server.abort();
    }
}

pub(crate) fn unique_name(prefix: &str) -> String {
    format!("{}-{}", prefix, crate::utils::random_text(8))
}
