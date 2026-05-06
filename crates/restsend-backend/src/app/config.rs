#[derive(Debug, Clone)]
pub struct AppConfig {
    pub addr: String,
    pub endpoint: String,
    pub database_url: String,
    pub openapi_schema: String,
    pub openapi_prefix: String,
    pub api_prefix: String,
    pub log_file: String,
    pub openapi_token: Option<String>,
    pub run_migrations: bool,
    pub migrate_only: bool,
    pub webhook_timeout_secs: u64,
    pub webhook_retries: usize,
    pub webhook_targets: Vec<String>,
    pub event_bus_size: usize,
    pub message_worker_count: usize,
    pub message_queue_size: usize,
    pub push_worker_count: usize,
    pub push_queue_size: usize,
    pub webhook_worker_count: usize,
    pub webhook_queue_size: usize,
    pub max_upload_bytes: usize,
    pub presence_backend: String,
    pub presence_node_id: String,
    pub presence_ttl_secs: u64,
    pub presence_heartbeat_secs: u64,
    pub ws_per_user_limit: usize,
    pub ws_client_queue_size: usize,
    pub ws_typing_interval_ms: u64,
    pub ws_drop_on_backpressure: bool,
    pub demo: bool,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self::from_env_and_args(std::env::args()).unwrap_or_else(|err| {
            eprintln!("restsend-backend argument error: {err}");
            std::process::exit(2);
        })
    }

    pub fn from_env_and_args(
        args: impl IntoIterator<Item = String>,
    ) -> Result<Self, String> {
        let addr_override = parse_addr_arg(args)?;
        let addr = addr_override.unwrap_or_else(|| {
            std::env::var("RS_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string())
        });
        let endpoint =
            build_endpoint(std::env::var("RS_ENDPOINT").unwrap_or_else(|_| addr.clone()));
        let database_url = std::env::var("RS_DATABASE_URL")
            .unwrap_or_else(|_| "sqlite://restsend-server.db?mode=rwc".to_string());
        let openapi_schema = std::env::var("RS_OPENAPI_SCHEMA")
            .unwrap_or_else(|_| "http".to_string())
            .trim()
            .to_ascii_lowercase();
        let openapi_prefix = normalize_path(
            std::env::var("RS_OPENAPI_PREFIX").unwrap_or_else(|_| "/open".to_string()),
        );
        let api_prefix =
            normalize_path(std::env::var("RS_API_PREFIX").unwrap_or_else(|_| "/api".to_string()));
        let log_file = std::env::var("RS_LOG_FILE")
            .unwrap_or_else(|_| "logs/restsend-backend.log".to_string());
        let openapi_token = std::env::var("RS_OPENAPI_TOKEN")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        let run_migrations = env_bool("RS_RUN_MIGRATIONS", true);
        let migrate_only = env_bool("RS_MIGRATE_ONLY", false);
        let webhook_timeout_secs = std::env::var("RS_WEBHOOK_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(10);
        let webhook_retries = std::env::var("RS_WEBHOOK_RETRIES")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(3);
        let webhook_targets = std::env::var("RS_WEBHOOK_TARGETS")
            .ok()
            .map(|v| {
                v.split(',')
                    .map(|item| item.trim().to_string())
                    .filter(|item| !item.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let event_bus_size = std::env::var("RS_EVENT_BUS_SIZE")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(1024);
        let message_worker_count = std::env::var("RS_MESSAGE_WORKERS")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(4);
        let message_queue_size = std::env::var("RS_MESSAGE_QUEUE_SIZE")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(1024);
        let push_worker_count = std::env::var("RS_PUSH_WORKERS")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(message_worker_count);
        let push_queue_size = std::env::var("RS_PUSH_QUEUE_SIZE")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(message_queue_size);
        let webhook_worker_count = std::env::var("RS_WEBHOOK_WORKERS")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(message_worker_count);
        let webhook_queue_size = std::env::var("RS_WEBHOOK_QUEUE_SIZE")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(message_queue_size);
        let max_upload_bytes = std::env::var("RS_MAX_UPLOAD_BYTES")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(10 * 1024 * 1024)
            .max(1024);
        let presence_backend = std::env::var("RS_PRESENCE_BACKEND")
            .unwrap_or_else(|_| "memory".to_string())
            .to_ascii_lowercase();
        let presence_node_id = std::env::var("RS_NODE_ID")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| format!("node-{}", uuid::Uuid::new_v4().simple()));
        let presence_ttl_secs = std::env::var("RS_PRESENCE_TTL_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(90)
            .max(10);
        let presence_heartbeat_secs = std::env::var("RS_PRESENCE_HEARTBEAT_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(30)
            .max(1)
            .min(presence_ttl_secs);
        let ws_per_user_limit = std::env::var("RS_WS_PER_USER_LIMIT")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(0);
        let ws_client_queue_size = std::env::var("RS_WS_CLIENT_QUEUE_SIZE")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(0);
        let ws_typing_interval_ms = std::env::var("RS_WS_TYPING_INTERVAL_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(1000);
        let ws_drop_on_backpressure = env_bool("RS_WS_DROP_ON_BACKPRESSURE", true);
        let demo = env_bool("RS_DEMO", false);

        Ok(Self {
            addr,
            endpoint,
            database_url,
            openapi_schema,
            openapi_prefix,
            api_prefix,
            log_file,
            openapi_token,
            run_migrations,
            migrate_only,
            webhook_timeout_secs,
            webhook_retries,
            webhook_targets,
            event_bus_size,
            message_worker_count,
            message_queue_size,
            push_worker_count,
            push_queue_size,
            webhook_worker_count,
            webhook_queue_size,
            max_upload_bytes,
            presence_backend,
            presence_node_id,
            presence_ttl_secs,
            presence_heartbeat_secs,
            ws_per_user_limit,
            ws_client_queue_size,
            ws_typing_interval_ms,
            ws_drop_on_backpressure,
            demo,
        })
    }
}

fn parse_addr_arg(args: impl IntoIterator<Item = String>) -> Result<Option<String>, String> {
    let mut args = args.into_iter();
    let _ = args.next();

    let mut addr = None;
    while let Some(arg) = args.next() {
        if let Some(value) = arg
            .strip_prefix("-addr=")
            .or_else(|| arg.strip_prefix("--addr="))
        {
            if value.trim().is_empty() {
                return Err("--addr requires a non-empty value".to_string());
            }
            addr = Some(value.to_string());
            continue;
        }

        if arg == "-addr" || arg == "--addr" {
            let Some(value) = args.next() else {
                return Err("--addr requires a value".to_string());
            };
            if value.trim().is_empty() {
                return Err("--addr requires a non-empty value".to_string());
            }
            addr = Some(value);
        }
    }

    Ok(addr)
}

fn build_endpoint(addr: String) -> String {
    let mut parts = addr.splitn(2, ':');
    let host = parts.next().unwrap_or_default();
    let Some(port) = parts.next() else {
        return String::new();
    };
    let host = if host.is_empty() { "127.0.0.1" } else { host };
    format!("{host}:{port}")
}

fn env_bool(name: &str, default: bool) -> bool {
    match std::env::var(name) {
        Ok(v) => matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"),
        Err(_) => default,
    }
}

fn normalize_path(v: String) -> String {
    if v.starts_with('/') {
        v
    } else {
        format!("/{v}")
    }
}

#[cfg(test)]
mod tests {
    use super::parse_addr_arg;

    #[test]
    fn parse_addr_arg_supports_separate_value() {
        let addr = parse_addr_arg([
            "restsend-backend".to_string(),
            "-addr".to_string(),
            "127.0.0.1:9000".to_string(),
        ])
        .unwrap();

        assert_eq!(addr.as_deref(), Some("127.0.0.1:9000"));
    }

    #[test]
    fn parse_addr_arg_supports_equals_value() {
        let addr = parse_addr_arg([
            "restsend-backend".to_string(),
            "--addr=127.0.0.1:9000".to_string(),
        ])
        .unwrap();

        assert_eq!(addr.as_deref(), Some("127.0.0.1:9000"));
    }

    #[test]
    fn parse_addr_arg_supports_double_dash_separate_value() {
        let addr = parse_addr_arg([
            "restsend-backend".to_string(),
            "--addr".to_string(),
            "127.0.0.1:9000".to_string(),
        ])
        .unwrap();

        assert_eq!(addr.as_deref(), Some("127.0.0.1:9000"));
    }

    #[test]
    fn parse_addr_arg_requires_value() {
        let err = parse_addr_arg(["restsend-backend".to_string(), "--addr".to_string()])
            .unwrap_err();

        assert_eq!(err, "--addr requires a value");
    }
}
