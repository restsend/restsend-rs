mod config;
pub mod state;

use axum::routing::{get, post};
use axum::Router;
use std::collections::HashSet;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use crate::api;
use crate::api::admin::hinit_static_path;
use crate::infra::db::{connect_db, run_migrations};
use crate::infra::event::{BackendEvent, EventBus};
use crate::infra::metrics::RuntimeMetrics;
use crate::infra::presence::{DbPresenceStore, MemoryPresenceStore, PresenceHub, PresenceStore};
use crate::infra::task_pool::TaskPool;
use crate::infra::webhook::WebhookSender;
use crate::infra::websocket::WsHub;
use crate::services::{
    AuthService, ChatService, ConversationService, RelationService, TopicService, UserService,
};

pub use config::AppConfig;
pub use state::AppState;

pub async fn build_router(
    config: AppConfig,
) -> Result<(Router<AppState>, AppState), sea_orm::DbErr> {
    let db = connect_db(&config.database_url).await?;
    if config.run_migrations {
        run_migrations(&db).await?;
    }
    if config.demo {
        create_demo_accounts(&db).await?;
    }

    let ws_hub = std::sync::Arc::new(WsHub::default());
    let presence_store: std::sync::Arc<dyn PresenceStore> = match config.presence_backend.as_str() {
        "db" => std::sync::Arc::new(DbPresenceStore::new(
            db.clone(),
            config.presence_node_id.clone(),
            config.endpoint.clone(),
            config.presence_ttl_secs,
        )),
        _ => std::sync::Arc::new(MemoryPresenceStore::new(config.presence_ttl_secs)),
    };
    let presence_hub = std::sync::Arc::new(PresenceHub::new(presence_store));
    let message_pool = std::sync::Arc::new(TaskPool::new(
        config.message_worker_count,
        config.message_queue_size,
    ));
    let push_pool = std::sync::Arc::new(TaskPool::new(
        config.push_worker_count,
        config.push_queue_size,
    ));
    let webhook_pool = std::sync::Arc::new(TaskPool::new(
        config.webhook_worker_count,
        config.webhook_queue_size,
    ));
    let event_bus = std::sync::Arc::new(EventBus::new(config.event_bus_size));
    let metrics = std::sync::Arc::new(RuntimeMetrics::default());
    let webhook_sender = std::sync::Arc::new(WebhookSender::new(
        config.webhook_timeout_secs,
        config.webhook_retries,
    ));
    let user_service = std::sync::Arc::new(UserService::new(db.clone()));
    let auth_service = std::sync::Arc::new(AuthService::new(db.clone()));
    let relation_service = std::sync::Arc::new(RelationService::new(db.clone()));
    let topic_service = std::sync::Arc::new(TopicService::new(db.clone()));
    let conversation_service = std::sync::Arc::new(ConversationService::new(db.clone()));
    let chat_service = std::sync::Arc::new(ChatService::new(db.clone()));

    let state = AppState {
        config: config.clone(),
        db,
        ws_hub,
        presence_hub,
        message_pool,
        push_pool,
        webhook_pool,
        event_bus,
        metrics,
        webhook_sender,
        cluster_push_client: reqwest::Client::new(),
        webhook_targets: std::sync::Arc::new(config.webhook_targets.clone()),
        user_service,
        auth_service,
        relation_service,
        topic_service,
        conversation_service,
        chat_service,
    };

    start_webhook_worker(state.clone());
    state
        .presence_hub
        .start_cleanup_loop(config.presence_heartbeat_secs);

    let openapi = Router::new()
        .route("/user/online/:userid", post(api::openapi::user_online))
        .route("/user/push/:userid", post(api::openapi::user_push))
        .route(
            "/user/push/:userid/:cid",
            post(api::openapi::user_push_with_cid),
        )
        .route("/user/register/:userid", post(api::openapi::user_register))
        .route("/user/list", post(api::openapi::user_list))
        .route("/user/auth/:userid", post(api::openapi::user_auth))
        .route("/user/update/:userid", post(api::openapi::user_update))
        .route(
            "/user/enabled/:userid",
            post(api::openapi::user_set_enabled),
        )
        .route("/user/staff/:userid", post(api::openapi::user_set_staff))
        .route(
            "/user/relation/:userid/:targetid",
            post(api::openapi::user_relation_update),
        )
        .route("/user/delete/:userid", post(api::openapi::user_deactive))
        .route(
            "/user/blacklist/get/:userid",
            post(api::openapi::user_blacklist_get),
        )
        .route(
            "/user/blacklist/add/:userid",
            post(api::openapi::user_blacklist_add),
        )
        .route(
            "/user/blacklist/remove/:userid",
            post(api::openapi::user_blacklist_remove),
        )
        .route("/topic/create", post(api::openapi::topic_create_auto))
        .route("/topic/create/:topicid", post(api::openapi::topic_create))
        .route("/topic/list", post(api::openapi::topic_list))
        .route("/topic/info/:topicid", post(api::openapi::topic_info))
        .route("/topic/update/:topicid", post(api::openapi::topic_update))
        .route(
            "/topic/enabled/:topicid",
            post(api::openapi::topic_set_enabled),
        )
        .route(
            "/topic/update_extra/:topicid",
            post(api::openapi::topic_update_extra),
        )
        .route("/topic/logs/:topicid", post(api::openapi::topic_logs))
        .route(
            "/topic/import/:topicid",
            post(api::openapi::topic_import_message),
        )
        .route(
            "/topic/send/:topicid",
            post(api::openapi::topic_send_message),
        )
        .route(
            "/topic/send/:topicid/:format",
            post(api::openapi::topic_send_message_with_format),
        )
        .route("/chat/:senderid", post(api::openapi::chat_send_message))
        .route(
            "/chat/:senderid/:format",
            post(api::openapi::chat_send_message_with_format),
        )
        .route("/topic/members/:topicid", post(api::openapi::topic_members))
        .route("/topic/join/:topicid", post(api::openapi::topic_join))
        .route("/topic/quit/:topicid", post(api::openapi::topic_quit))
        .route("/topic/dismiss/:topicid", post(api::openapi::topic_dismiss))
        .route(
            "/topic/member/:topicid/:userid",
            post(api::openapi::topic_update_member),
        )
        .route(
            "/topic/member_info/:topicid/:userid",
            post(api::openapi::topic_member_info),
        )
        .route(
            "/topic/kickout/:topicid/:userid",
            post(api::openapi::topic_kickout_member),
        )
        .route(
            "/topic/transfer/:topicid/:userid",
            post(api::openapi::topic_transfer_owner),
        )
        .route(
            "/topic/admin/add/:topicid/:userid",
            post(api::openapi::topic_add_admin),
        )
        .route(
            "/topic/admin/remove/:topicid/:userid",
            post(api::openapi::topic_remove_admin),
        )
        .route(
            "/topic/silent/member/:topicid",
            post(api::openapi::topic_silent_member),
        )
        .route(
            "/topic/silent/whitelist/add/:topicid",
            post(api::openapi::topic_add_silent_whitelist),
        )
        .route(
            "/topic/silent/whitelist/remove/:topicid",
            post(api::openapi::topic_remove_silent_whitelist),
        )
        .route(
            "/topic/silent/topic/:topicid",
            post(api::openapi::topic_silent),
        )
        .route(
            "/conversation/info/:userid/:topicid",
            post(api::openapi::conversation_info),
        )
        .route(
            "/conversation/remove/:userid/:topicid",
            post(api::openapi::conversation_remove),
        )
        .route(
            "/conversation/unread/:userid/:topicid",
            post(api::openapi::conversation_mark_unread),
        )
        .route(
            "/conversation/update/:userid/:topicid",
            post(api::openapi::conversation_update),
        )
        .route("/docs", get(api::openapi::docs));

    let api_public = Router::new()
        .route("/health", get(api::health::health))
        .route("/live", get(api::health::live))
        .route("/ready", get(api::health::ready))
        .route("/guest/login", post(api::auth::guest_login));

    let api_protected = Router::new()
        .route("/devices", get(api::user::devices))
        .route("/connect", get(api::routes_ws::ws_connect))
        .route("/kick/:cid", post(api::user::kick))
        .route("/attachment/upload", post(api::attachment::upload))
        .route(
            "/attachment/*filepath",
            get(api::attachment::get_attachment),
        )
        .route("/profile", post(api::user::profiles))
        .route("/profile/:userid", post(api::user::single_profile))
        .route("/profile/update", post(api::user::update_profile))
        .route("/relation/:userid", post(api::user::update_relation))
        .route("/chat/list", post(api::chat::chat_list))
        .route("/chat/info/:topicid", post(api::chat::chat_info))
        .route("/chat/remove/:topicid", post(api::chat::chat_remove))
        .route("/chat/update/:topicid", post(api::chat::chat_update))
        .route("/chat/read/:topicid", post(api::chat::chat_read))
        .route("/chat/unread/:topicid", post(api::chat::chat_unread))
        .route("/chat/readall", post(api::chat::chat_read_all))
        .route("/chat/sync/:topicid", post(api::chat::chat_sync))
        .route("/chat/batch_sync", post(api::chat::chat_batch_sync))
        .route("/chat/send", post(api::chat::chat_send))
        .route("/chat/send/:topicid", post(api::chat::chat_send_to_topic))
        .route(
            "/chat/create/:userid",
            post(api::chat::chat_create_with_user),
        )
        .route(
            "/chat/remove_messages/:topicid",
            post(api::chat::chat_remove_messages),
        )
        .route(
            "/chat/clear_messages/:topicid",
            post(api::chat::chat_clear_messages),
        )
        .route("/topic/info/:topicid", post(api::topic::topic_info))
        .route("/topic/create", post(api::topic::topic_create))
        .route(
            "/topic/create/:userid",
            post(api::topic::topic_create_with_user),
        )
        .route("/topic/members/:topicid", post(api::topic::topic_members))
        .route("/topic/dismiss/:topicid", post(api::topic::topic_dismiss))
        .route("/topic/quit/:topicid", post(api::topic::topic_quit))
        .route("/topic/knock/:topicid", post(api::topic::topic_knock))
        .route(
            "/topic/invite/:topicid/:userid",
            post(api::topic::topic_invite),
        )
        .route(
            "/topic/admin/add_member/:topicid/:userid",
            post(api::topic::topic_admin_add_member),
        )
        .route(
            "/topic/admin/update/:topicid",
            post(api::topic::topic_admin_update),
        )
        .route(
            "/topic/admin/transfer/:topicid/:userid",
            post(api::topic::topic_admin_transfer_owner),
        )
        .route(
            "/topic/admin/add_admin/:topicid/:userid",
            post(api::topic::topic_admin_add_admin),
        )
        .route(
            "/topic/admin/remove_admin/:topicid/:userid",
            post(api::topic::topic_admin_remove_admin),
        )
        .route(
            "/topic/admin/list_knock/:topicid",
            post(api::topic::topic_admin_list_knock),
        )
        .route(
            "/topic/admin/knock/accept/:topicid/:userid",
            post(api::topic::topic_admin_accept_knock),
        )
        .route(
            "/topic/admin/knock/reject/:topicid/:userid",
            post(api::topic::topic_admin_reject_knock),
        )
        .route(
            "/topic/admin/notice/:topicid",
            post(api::topic::topic_admin_notice),
        )
        .route(
            "/topic/admin/kickout/:topicid/:userid",
            post(api::topic::topic_admin_kickout),
        )
        .route(
            "/topic/admin/silent/:topicid/:userid",
            post(api::topic::topic_admin_silent_user),
        )
        .route(
            "/topic/admin/silent_topic/:topicid/:userid",
            post(api::topic::topic_admin_silent_user),
        )
        .route(
            "/topic/admin/silent_topic/:topicid",
            post(api::topic::topic_admin_silent_topic),
        )
        .route("/list_blocked", post(api::user::list_blocked))
        .route("/block/:userid", post(api::user::block_user))
        .route("/unblock/:userid", post(api::user::unblock_user))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            api::middleware_auth::user_auth,
        ));

    let api_router = api_public.merge(api_protected);

    let admin_enabled = hinit_static_path("admin.html").is_some();
    let chat_enabled = hinit_static_path("chat.html").is_some();

    let mut app = Router::new();
    if admin_enabled {
        app = app
            .route("/admin", get(api::admin::spa))
            .route(
                "/admin/api/config",
                get(api::admin::config_view).route_layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    api::middleware_auth::openapi_auth,
                )),
            )
            .route(
                "/admin/api/bootstrap",
                get(api::admin::bootstrap_state).post(api::admin::bootstrap_init),
            )
            .route(
                "/admin/api/perf",
                get(api::admin::perf_stats).route_layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    api::middleware_auth::openapi_auth,
                )),
            );
    }
    if chat_enabled {
        app = app.route("/chat", get(api::admin::chat_spa));
        for p in [".", "..", "../.."] {
            let js_dir = std::path::Path::new(p).join("js");
            if js_dir.exists() {
                app = app.nest_service("/js", ServeDir::new(js_dir));
                break;
            }
        }
    }
    if config.demo {
        app = app
            .route("/", get(api::admin::demo_spa))
            .route("/chat/api/demo-users", get(api::admin::demo_users));
    }

    let app = app
        .route("/auth/register", post(api::auth::register))
        .route("/auth/login", post(api::auth::login))
        .route(
            "/auth/logout",
            get(api::auth::logout).route_layer(axum::middleware::from_fn_with_state(
                state.clone(),
                api::middleware_auth::user_auth,
            )),
        )
        .nest(
            &config.openapi_prefix,
            openapi.layer(axum::middleware::from_fn_with_state(
                state.clone(),
                api::middleware_auth::openapi_auth,
            )),
        )
        .nest(&config.api_prefix, api_router)
        .route("/ws", get(api::routes_ws::ws_upgrade))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            api::access_log::request_access_log,
        ))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    Ok((app, state))
}

fn start_webhook_worker(state: AppState) {
    let mut rx = state.event_bus.subscribe();
    tokio::spawn(async move {
        loop {
            let event = match rx.recv().await {
                Ok(event) => event,
                Err(err) => {
                    tracing::warn!(error = %err, "event bus receive failed");
                    continue;
                }
            };

            let state = state.clone();
            let webhook_pool = state.webhook_pool.clone();
            if let Err(err) = webhook_pool
                .submit(async move {
                    handle_event_webhooks(state, event).await;
                })
                .await
            {
                tracing::warn!(error = %err, "webhook task submit failed");
            }
        }
    });
}

async fn handle_event_webhooks(state: AppState, event: BackendEvent) {
    if !event.should_send_webhook() {
        return;
    }

    let event_name = event.event_name();
    let topic_id = event.topic_id().map(|v| v.to_string());
    let data = event.data_payload();

    let mut targets: Vec<String> = state.webhook_targets.as_ref().clone();
    targets.extend(event.explicit_webhooks().iter().cloned());
    if event.use_topic_webhooks() {
        if let Some(topic_id) = topic_id.as_deref() {
            if let Ok(topic) = state.topic_service.get_by_id(topic_id).await {
                targets.extend(topic.webhooks);
            }
        }
    }
    let mut seen = HashSet::new();
    targets.retain(|target| !target.trim().is_empty() && seen.insert(target.clone()));

    if targets.is_empty() {
        return;
    }

    tracing::info!(
        event = event_name,
        topic_id = ?topic_id,
        targets = targets.len(),
        "dispatch webhook event"
    );

    let payload = serde_json::json!({
        "name": event_name,
        "topicId": topic_id,
        "data": data,
    });
    for target in targets {
        let state = state.clone();
        let webhook_pool = state.webhook_pool.clone();
        let target = target.clone();
        let submit_target = target.clone();
        let topic_id = topic_id.clone();
        let payload = payload.clone();
        if let Err(err) = webhook_pool
            .submit(async move {
                let st = std::time::Instant::now();
                if let Err(err) = state.webhook_sender.send_json(&target, &payload).await {
                    state.metrics.incr_webhook_failures();
                    tracing::warn!(
                        target = %target,
                        event = event_name,
                        topic_id = ?topic_id,
                        elapsed_ms = st.elapsed().as_millis() as u64,
                        error = %err,
                        "send webhook failed"
                    );
                } else {
                    state.metrics.incr_webhook_deliveries();
                    tracing::info!(
                        target = %target,
                        event = event_name,
                        topic_id = ?topic_id,
                        elapsed_ms = st.elapsed().as_millis() as u64,
                        "webhook delivered"
                    );
                }
            })
            .await
        {
            tracing::warn!(target = %submit_target, event = event_name, error = %err, "webhook delivery task submit failed");
        }
    }
}

async fn create_demo_accounts(db: &sea_orm::DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    use crate::entity::user;
    use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait, IntoActiveModel};

    let demo_users = [
        ("alice", "Alice"),
        ("bob", "Bob"),
        ("guido", "Guido"),
        ("jinti", "Jinti"),
    ];

    for (user_id, display_name) in &demo_users {
        let now = chrono::Utc::now().to_rfc3339();
        let password = format!("{}:demo", user_id);
        let hashed = crate::api::auth::hash_password(&password);
        let existing = user::Entity::find_by_id(user_id.to_string())
            .one(db)
            .await?;
        if let Some(model) = existing {
            let mut active = model.into_active_model();
            active.password = Set(hashed);
            active.display_name = Set(display_name.to_string());
            active.updated_at = Set(now);
            active.update(db).await?;
            tracing::info!(user_id = %user_id, "demo account password reset");
        } else {
            let active = user::ActiveModel {
                user_id: Set(user_id.to_string()),
                password: Set(hashed),
                display_name: Set(display_name.to_string()),
                avatar: Set(String::new()),
                source: Set("demo".to_string()),
                locale: Set(String::new()),
                city: Set(String::new()),
                country: Set(String::new()),
                gender: Set(String::new()),
                public_key: Set(String::new()),
                is_staff: Set(false),
                enabled: Set(true),
                created_at: Set(now.clone()),
                updated_at: Set(now),
            };
            active.insert(db).await?;
            tracing::info!(user_id = %user_id, "demo account created");
        }
    }

    // create demo admin account with fixed credentials
    {
        let now = chrono::Utc::now().to_rfc3339();
        let user_id = "admin";
        let password = "restsend";
        let hashed = crate::api::auth::hash_password(password);
        let existing = user::Entity::find_by_id(user_id.to_string())
            .one(db)
            .await?;
        if let Some(model) = existing {
            let mut active = model.into_active_model();
            active.password = Set(hashed);
            active.is_staff = Set(true);
            active.updated_at = Set(now);
            active.update(db).await?;
            tracing::info!(user_id = %user_id, "demo admin password reset");
        } else {
            let active = user::ActiveModel {
                user_id: Set(user_id.to_string()),
                password: Set(hashed),
                display_name: Set("Admin".to_string()),
                avatar: Set(String::new()),
                source: Set("demo".to_string()),
                locale: Set(String::new()),
                city: Set(String::new()),
                country: Set(String::new()),
                gender: Set(String::new()),
                public_key: Set(String::new()),
                is_staff: Set(true),
                enabled: Set(true),
                created_at: Set(now.clone()),
                updated_at: Set(now),
            };
            active.insert(db).await?;
            tracing::info!(user_id = %user_id, "demo admin account created");
        }
    }

    Ok(())
}

pub fn init_tracing(config: &AppConfig) -> Option<WorkerGuard> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let log_path = std::path::Path::new(&config.log_file);
    if let Some(parent) = log_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let file_appender = tracing_appender::rolling::never(
        log_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new(".")),
        log_path
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("restsend-backend.log"),
    );
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);

    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .compact()
        .with_writer(std::io::stdout);
    let file_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_target(true)
        .with_writer(file_writer);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(stdout_layer)
        .with(file_layer)
        .init();
    Some(guard)
}
