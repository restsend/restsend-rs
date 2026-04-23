#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use futures_util::{SinkExt, StreamExt};
    use http_body_util::BodyExt;
    use image::GenericImageView;
    use sea_orm::{ActiveModelTrait, EntityTrait};
    use tokio::net::TcpListener;
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    use tower::util::ServiceExt;
    use uuid::Uuid;

    use crate::app::{build_router, AppConfig};

    fn test_config() -> AppConfig {
        AppConfig {
            addr: "127.0.0.1:0".to_string(),
            endpoint: "127.0.0.1:0".to_string(),
            database_url: format!(
                "sqlite:file:restsend-test-{}?mode=memory&cache=shared",
                Uuid::new_v4().simple()
            ),
            openapi_schema: "http".to_string(),
            openapi_prefix: "/open".to_string(),
            api_prefix: "/api".to_string(),
            log_file: format!("logs/test-{}.log", Uuid::new_v4().simple()),
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
            presence_node_id: "test-node".to_string(),
            presence_ttl_secs: 90,
            presence_heartbeat_secs: 10,
            ws_per_user_limit: 0,
            ws_client_queue_size: 0,
            ws_typing_interval_ms: 1000,
            ws_drop_on_backpressure: true,
        }
    }

    #[tokio::test]
    async fn health_endpoint_works() {
        let (app, state) = build_router(test_config()).await.expect("build router");
        let app = app.with_state(state);

        let req = Request::builder()
            .uri("/api/health")
            .method("GET")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains("restsend-backend"));
    }

    #[tokio::test]
    async fn attachment_upload_private_access_and_external_redirect_match_go_basics() {
        let (app, state) = build_router(test_config()).await.expect("build router");
        let owner_token =
            register_and_auth(&app.clone().with_state(state.clone()), "bob@unittest").await;
        let viewer_token =
            register_and_auth(&app.clone().with_state(state.clone()), "alice@unittest").await;
        let app = app.with_state(state.clone());

        let boundary = "X-BOUNDARY";
        let body = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"unittest.go\"\r\nContent-Type: application/octet-stream\r\n\r\npackage aigcapi\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"private\"\r\n\r\ntrue\r\n--{boundary}--\r\n"
        );
        let upload_req = Request::builder()
            .uri("/api/attachment/upload")
            .method("POST")
            .header("Authorization", format!("Bearer {owner_token}"))
            .header(
                "content-type",
                format!("multipart/form-data; boundary={boundary}"),
            )
            .body(Body::from(body))
            .unwrap();
        let upload_resp = app.clone().oneshot(upload_req).await.unwrap();
        assert_eq!(upload_resp.status(), StatusCode::OK);
        let upload_body = upload_resp.into_body().collect().await.unwrap().to_bytes();
        let upload_json: serde_json::Value = serde_json::from_slice(&upload_body).unwrap();
        let public_url = upload_json
            .get("publicUrl")
            .and_then(|v| v.as_str())
            .unwrap()
            .to_string();

        let owner_get_req = Request::builder()
            .uri(&public_url)
            .method("GET")
            .header("Authorization", format!("Bearer {owner_token}"))
            .body(Body::empty())
            .unwrap();
        let owner_get_resp = app.clone().oneshot(owner_get_req).await.unwrap();
        assert_eq!(owner_get_resp.status(), StatusCode::OK);
        let owner_get_body = owner_get_resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        assert_eq!(
            String::from_utf8(owner_get_body.to_vec()).unwrap(),
            "package aigcapi"
        );

        let viewer_get_req = Request::builder()
            .uri(&public_url)
            .method("GET")
            .header("Authorization", format!("Bearer {viewer_token}"))
            .body(Body::empty())
            .unwrap();
        let viewer_get_resp = app.clone().oneshot(viewer_get_req).await.unwrap();
        assert_eq!(viewer_get_resp.status(), StatusCode::UNAUTHORIZED);

        let png = {
            let mut bytes = Vec::new();
            image::DynamicImage::ImageRgba8(image::RgbaImage::new(1024, 512))
                .write_to(
                    &mut std::io::Cursor::new(&mut bytes),
                    image::ImageFormat::Png,
                )
                .unwrap();
            bytes
        };
        let boundary = "IMG-BOUNDARY";
        let mut image_body = Vec::new();
        image_body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"unittest.png\"\r\nContent-Type: image/png\r\n\r\n"
            )
            .as_bytes(),
        );
        image_body.extend_from_slice(&png);
        image_body.extend_from_slice(
            format!(
                "\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"private\"\r\n\r\ntrue\r\n--{boundary}--\r\n"
            )
            .as_bytes(),
        );
        let image_upload_req = Request::builder()
            .uri("/api/attachment/upload")
            .method("POST")
            .header("Authorization", format!("Bearer {owner_token}"))
            .header(
                "content-type",
                format!("multipart/form-data; boundary={boundary}"),
            )
            .body(Body::from(image_body))
            .unwrap();
        let image_upload_resp = app.clone().oneshot(image_upload_req).await.unwrap();
        assert_eq!(image_upload_resp.status(), StatusCode::OK);
        let image_upload_body = image_upload_resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let image_upload_json: serde_json::Value =
            serde_json::from_slice(&image_upload_body).unwrap();
        let image_public_url = image_upload_json
            .get("publicUrl")
            .and_then(|v| v.as_str())
            .unwrap()
            .to_string();

        let thumb_req = Request::builder()
            .uri(format!("{image_public_url}?size=sm"))
            .method("GET")
            .header("Authorization", format!("Bearer {owner_token}"))
            .body(Body::empty())
            .unwrap();
        let thumb_resp = app.clone().oneshot(thumb_req).await.unwrap();
        assert_eq!(thumb_resp.status(), StatusCode::OK);
        assert_eq!(
            thumb_resp
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok()),
            Some("image/jpeg")
        );
        let thumb_body = thumb_resp.into_body().collect().await.unwrap().to_bytes();
        let thumb = image::load_from_memory(&thumb_body).unwrap();
        assert_eq!(thumb.dimensions(), (512, 256));

        let huge_thumb_req = Request::builder()
            .uri(format!("{image_public_url}?size=999999"))
            .method("GET")
            .header("Authorization", format!("Bearer {owner_token}"))
            .body(Body::empty())
            .unwrap();
        let huge_thumb_resp = app.clone().oneshot(huge_thumb_req).await.unwrap();
        assert_eq!(huge_thumb_resp.status(), StatusCode::OK);
        let huge_thumb_body = huge_thumb_resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let huge_thumb = image::load_from_memory(&huge_thumb_body).unwrap();
        assert_eq!(huge_thumb.dimensions(), (1024, 512));

        let external = crate::entity::attachment::ActiveModel {
            path: sea_orm::ActiveValue::Set("test.png".to_string()),
            file_name: sea_orm::ActiveValue::Set("test.png".to_string()),
            store_path: sea_orm::ActiveValue::Set("http://server/test.png?test".to_string()),
            owner_id: sea_orm::ActiveValue::Set("bob@unittest".to_string()),
            topic_id: sea_orm::ActiveValue::Set(String::new()),
            size: sea_orm::ActiveValue::Set(1024),
            ext: sea_orm::ActiveValue::Set(".png".to_string()),
            private: sea_orm::ActiveValue::Set(false),
            external: sea_orm::ActiveValue::Set(true),
            tags: sea_orm::ActiveValue::Set(String::new()),
            remark: sea_orm::ActiveValue::Set(String::new()),
            created_at: sea_orm::ActiveValue::Set("2026-04-23T00:00:00Z".to_string()),
        };
        let _ = external.insert(&state.db).await.unwrap();

        let external_req = Request::builder()
            .uri("/api/attachment/test.png?size=sm")
            .method("GET")
            .header("Authorization", format!("Bearer {owner_token}"))
            .body(Body::empty())
            .unwrap();
        let external_resp = app.oneshot(external_req).await.unwrap();
        assert_eq!(external_resp.status(), StatusCode::FOUND);
        assert_eq!(
            external_resp
                .headers()
                .get("location")
                .and_then(|v| v.to_str().ok()),
            Some("http://server/test.png?test&size=sm")
        );
    }

    #[tokio::test]
    async fn openapi_requires_auth() {
        let (app, state) = build_router(test_config()).await.expect("build router");
        let app = app.with_state(state);

        let req = Request::builder()
            .uri("/open/user/online/u1")
            .method("POST")
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        let req_ok = Request::builder()
            .uri("/open/user/online/u1")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .body(Body::empty())
            .unwrap();
        let resp_ok = app.oneshot(req_ok).await.unwrap();
        assert_eq!(resp_ok.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn custom_api_and_openapi_prefixes_are_supported() {
        let mut config = test_config();
        config.api_prefix = "/test/api".to_string();
        config.openapi_prefix = "/test/openapi".to_string();

        let (app, state) = build_router(config).await.expect("build router");
        let app = app.with_state(state);

        let register_req = Request::builder()
            .uri("/test/openapi/user/register/bob@unittest")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .unwrap();
        let register_resp = app.clone().oneshot(register_req).await.unwrap();
        assert_eq!(register_resp.status(), StatusCode::OK);

        let auth_req = Request::builder()
            .uri("/test/openapi/user/auth/bob@unittest")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .unwrap();
        let auth_resp = app.clone().oneshot(auth_req).await.unwrap();
        assert_eq!(auth_resp.status(), StatusCode::OK);
        let auth_body = auth_resp.into_body().collect().await.unwrap().to_bytes();
        let owner_token = extract_token(std::str::from_utf8(&auth_body).unwrap()).unwrap();

        let profile_req = Request::builder()
            .uri("/test/api/profile")
            .method("POST")
            .header("Authorization", format!("Bearer {owner_token}"))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"userIds":["bob@unittest"]}"#))
            .unwrap();
        let profile_resp = app.clone().oneshot(profile_req).await.unwrap();
        assert_eq!(profile_resp.status(), StatusCode::OK);

        let openapi_req = Request::builder()
            .uri("/test/openapi/user/online/bob@unittest")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .body(Body::empty())
            .unwrap();
        let openapi_resp = app.oneshot(openapi_req).await.unwrap();
        assert_eq!(openapi_resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn user_register_and_auth_roundtrip() {
        let (app, state) = build_router(test_config()).await.expect("build router");
        let app = app.with_state(state);

        let register_req = Request::builder()
            .uri("/open/user/register/alice")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"displayName":"Alice","city":"Shanghai"}"#))
            .unwrap();
        let register_resp = app.clone().oneshot(register_req).await.unwrap();
        let register_status = register_resp.status();
        let register_body = register_resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let _register_text = String::from_utf8(register_body.to_vec()).unwrap();
        assert_eq!(register_status, StatusCode::OK);

        let auth_req = Request::builder()
            .uri("/open/user/auth/alice")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"createWhenNotExist":false}"#))
            .unwrap();
        let auth_resp = app.oneshot(auth_req).await.unwrap();
        let auth_status = auth_resp.status();
        let body = auth_resp.into_body().collect().await.unwrap().to_bytes();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert_eq!(auth_status, StatusCode::OK);
        assert!(text.contains("authToken"));
        assert!(text.contains("alice"));
    }

    #[tokio::test]
    async fn sdk_auth_register_and_login_roundtrip() {
        let (app, state) = build_router(test_config()).await.expect("build router");
        let db = state.db.clone();
        let app = app.with_state(state);

        let register_req = Request::builder()
            .uri("/auth/register")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"email":"sdk-user","password":"sdk-pass","remember":true}"#,
            ))
            .unwrap();
        let register_resp = app.clone().oneshot(register_req).await.unwrap();
        assert_eq!(register_resp.status(), StatusCode::OK);
        let register_body = register_resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let register_json: serde_json::Value = serde_json::from_slice(&register_body).unwrap();
        assert_eq!(
            register_json.get("email").and_then(|v| v.as_str()),
            Some("sdk-user")
        );
        let stored_user = crate::entity::user::Entity::find_by_id("sdk-user".to_string())
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_ne!(stored_user.password, "sdk-pass");
        assert_eq!(
            stored_user.password,
            crate::api::auth::hash_password("sdk-pass")
        );
        let issued_token = register_json
            .get("token")
            .and_then(|v| v.as_str())
            .unwrap()
            .to_string();

        let login_req = Request::builder()
            .uri("/auth/login")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"email":"sdk-user","password":"sdk-pass","remember":true}"#,
            ))
            .unwrap();
        let login_resp = app.clone().oneshot(login_req).await.unwrap();
        assert_eq!(login_resp.status(), StatusCode::OK);
        let login_body = login_resp.into_body().collect().await.unwrap().to_bytes();
        let login_json: serde_json::Value = serde_json::from_slice(&login_body).unwrap();
        assert_eq!(
            login_json.get("email").and_then(|v| v.as_str()),
            Some("sdk-user")
        );
        assert!(login_json.get("token").and_then(|v| v.as_str()).is_some());

        let token_login_req = Request::builder()
            .uri("/auth/login")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(format!(
                r#"{{"email":"sdk-user","authToken":"{issued_token}","remember":true}}"#
            )))
            .unwrap();
        let token_login_resp = app.oneshot(token_login_req).await.unwrap();
        assert_eq!(token_login_resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn attachment_upload_rejects_oversized_payload() {
        let mut config = test_config();
        config.max_upload_bytes = 8;
        let (app, state) = build_router(config).await.expect("build router");
        let token =
            register_and_auth(&app.clone().with_state(state.clone()), "upload-limit-user").await;
        let app = app.with_state(state);

        let boundary = "UPLOAD-LIMIT";
        let body = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"oversized.txt\"\r\nContent-Type: text/plain\r\n\r\n123456789\r\n--{boundary}--\r\n"
        );
        let req = Request::builder()
            .uri("/api/attachment/upload")
            .method("POST")
            .header("Authorization", format!("Bearer {token}"))
            .header(
                "content-type",
                format!("multipart/form-data; boundary={boundary}"),
            )
            .body(Body::from(body))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn topic_send_and_logs_flow() {
        let (app, state) = build_router(test_config()).await.expect("build router");
        let app = app.with_state(state);

        let create_req = Request::builder()
            .uri("/open/topic/create/topic-flow")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"senderId":"alice","members":["alice","bob"]}"#,
            ))
            .unwrap();
        let create_resp = app.clone().oneshot(create_req).await.unwrap();
        assert_eq!(create_resp.status(), StatusCode::OK);

        let send_req = Request::builder()
            .uri("/open/topic/send/topic-flow")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"senderId":"alice","type":"chat","chatId":"m1","message":"hello"}"#,
            ))
            .unwrap();
        let send_resp = app.clone().oneshot(send_req).await.unwrap();
        assert_eq!(send_resp.status(), StatusCode::OK);

        let logs_req = Request::builder()
            .uri("/open/topic/logs/topic-flow")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"limit":10}"#))
            .unwrap();
        let logs_resp = app.clone().oneshot(logs_req).await.unwrap();
        assert_eq!(logs_resp.status(), StatusCode::OK);
        let body = logs_resp.into_body().collect().await.unwrap().to_bytes();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains("topic-flow"));
        assert!(text.contains("hello"));

        let members_req = Request::builder()
            .uri("/open/topic/members/topic-flow")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .body(Body::empty())
            .unwrap();
        let members_resp = app.clone().oneshot(members_req).await.unwrap();
        let members_status = members_resp.status();
        let members_body = members_resp.into_body().collect().await.unwrap().to_bytes();
        let members_json: serde_json::Value = serde_json::from_slice(&members_body).unwrap();
        assert_eq!(members_status, StatusCode::OK);
        assert!(members_json
            .get("items")
            .and_then(|v| v.as_array())
            .is_some());
    }

    #[tokio::test]
    async fn user_api_requires_user_token() {
        let (app, state) = build_router(test_config()).await.expect("build router");
        let app = app.with_state(state);

        let req = Request::builder()
            .uri("/api/devices")
            .method("GET")
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        let reg_req = Request::builder()
            .uri("/open/user/register/u-token")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .unwrap();
        let _ = app.clone().oneshot(reg_req).await.unwrap();

        let auth_req = Request::builder()
            .uri("/open/user/auth/u-token")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .unwrap();
        let auth_resp = app.clone().oneshot(auth_req).await.unwrap();
        let body = auth_resp.into_body().collect().await.unwrap().to_bytes();
        let text = String::from_utf8(body.to_vec()).unwrap();
        let token = extract_token(&text).expect("extract user token");

        let devices_req = Request::builder()
            .uri("/api/devices")
            .method("GET")
            .header("Authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();
        let devices_resp = app.clone().oneshot(devices_req).await.unwrap();
        assert_eq!(devices_resp.status(), StatusCode::OK);

        let logout_req = Request::builder()
            .uri("/auth/logout")
            .method("GET")
            .header("Authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();
        let logout_resp = app.clone().oneshot(logout_req).await.unwrap();
        assert_eq!(logout_resp.status(), StatusCode::OK);

        let devices_after_logout_req = Request::builder()
            .uri("/api/devices")
            .method("GET")
            .header("Authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();
        let devices_after_logout_resp = app.oneshot(devices_after_logout_req).await.unwrap();
        assert_eq!(devices_after_logout_resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn api_profile_update_and_block_list_match_go_behavior() {
        let (app, state) = build_router(test_config()).await.expect("build router");
        let app = app.with_state(state);

        let bob_token = register_and_auth(&app, "bob@unittest").await;
        let _alice_token = register_and_auth(&app, "alice@unittest").await;

        let profiles_req = Request::builder()
            .uri("/api/profile")
            .method("POST")
            .header("Authorization", format!("Bearer {bob_token}"))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"userIds":["alice@unittest"]}"#))
            .unwrap();
        let profiles_resp = app.clone().oneshot(profiles_req).await.unwrap();
        assert_eq!(profiles_resp.status(), StatusCode::OK);
        let profiles_body = profiles_resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let profiles_json: serde_json::Value = serde_json::from_slice(&profiles_body).unwrap();
        assert_eq!(profiles_json.as_array().map(|v| v.len()), Some(1));
        assert_eq!(
            profiles_json
                .as_array()
                .and_then(|v| v.first())
                .and_then(|v| v.get("userId"))
                .and_then(|v| v.as_str()),
            Some("alice@unittest")
        );

        let profiles_with_missing_req = Request::builder()
            .uri("/api/profile")
            .method("POST")
            .header("Authorization", format!("Bearer {bob_token}"))
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"userIds":["alice@unittest","alice2@unittest"]}"#,
            ))
            .unwrap();
        let profiles_with_missing_resp = app
            .clone()
            .oneshot(profiles_with_missing_req)
            .await
            .unwrap();
        assert_eq!(profiles_with_missing_resp.status(), StatusCode::OK);
        let profiles_with_missing_body = profiles_with_missing_resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let profiles_with_missing_json: serde_json::Value =
            serde_json::from_slice(&profiles_with_missing_body).unwrap();
        assert_eq!(
            profiles_with_missing_json.as_array().map(|v| v.len()),
            Some(1)
        );

        let update_profile_req = Request::builder()
            .uri("/api/profile/update")
            .method("POST")
            .header("Authorization", format!("Bearer {bob_token}"))
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"displayName":"Bob","avatar":"bob.png","gender":"NaN"}"#,
            ))
            .unwrap();
        let update_profile_resp = app.clone().oneshot(update_profile_req).await.unwrap();
        assert_eq!(update_profile_resp.status(), StatusCode::OK);

        let bob_profile_req = Request::builder()
            .uri("/api/profile/bob@unittest")
            .method("POST")
            .header("Authorization", format!("Bearer {bob_token}"))
            .body(Body::empty())
            .unwrap();
        let bob_profile_resp = app.clone().oneshot(bob_profile_req).await.unwrap();
        assert_eq!(bob_profile_resp.status(), StatusCode::OK);
        let bob_profile_body = bob_profile_resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let bob_profile_json: serde_json::Value =
            serde_json::from_slice(&bob_profile_body).unwrap();
        assert_eq!(
            bob_profile_json.get("name").and_then(|v| v.as_str()),
            Some("Bob")
        );
        assert_eq!(
            bob_profile_json.get("avatar").and_then(|v| v.as_str()),
            Some("bob.png")
        );
        assert_eq!(
            bob_profile_json.get("gender").and_then(|v| v.as_str()),
            Some("NaN")
        );

        let block_req = Request::builder()
            .uri("/api/block/alice@unittest")
            .method("POST")
            .header("Authorization", format!("Bearer {bob_token}"))
            .body(Body::empty())
            .unwrap();
        let block_resp = app.clone().oneshot(block_req).await.unwrap();
        assert_eq!(block_resp.status(), StatusCode::OK);

        let blocked_list_req = Request::builder()
            .uri("/api/list_blocked")
            .method("POST")
            .header("Authorization", format!("Bearer {bob_token}"))
            .body(Body::empty())
            .unwrap();
        let blocked_list_resp = app.clone().oneshot(blocked_list_req).await.unwrap();
        assert_eq!(blocked_list_resp.status(), StatusCode::OK);
        let blocked_list_body = blocked_list_resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let blocked_list_json: serde_json::Value =
            serde_json::from_slice(&blocked_list_body).unwrap();
        assert_eq!(blocked_list_json.as_array().map(|v| v.len()), Some(1));
        assert_eq!(
            blocked_list_json
                .as_array()
                .and_then(|v| v.first())
                .and_then(|v| v.as_str()),
            Some("alice@unittest")
        );

        let alice_profile_req = Request::builder()
            .uri("/api/profile/alice@unittest")
            .method("POST")
            .header("Authorization", format!("Bearer {bob_token}"))
            .body(Body::empty())
            .unwrap();
        let alice_profile_resp = app.clone().oneshot(alice_profile_req).await.unwrap();
        assert_eq!(alice_profile_resp.status(), StatusCode::OK);
        let alice_profile_body = alice_profile_resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let alice_profile_json: serde_json::Value =
            serde_json::from_slice(&alice_profile_body).unwrap();
        assert_eq!(
            alice_profile_json
                .get("isBlocked")
                .and_then(|v| v.as_bool()),
            Some(true)
        );

        let unblock_req = Request::builder()
            .uri("/api/unblock/alice@unittest")
            .method("POST")
            .header("Authorization", format!("Bearer {bob_token}"))
            .body(Body::empty())
            .unwrap();
        let unblock_resp = app.clone().oneshot(unblock_req).await.unwrap();
        assert_eq!(unblock_resp.status(), StatusCode::OK);

        let blocked_list_after_req = Request::builder()
            .uri("/api/list_blocked")
            .method("POST")
            .header("Authorization", format!("Bearer {bob_token}"))
            .body(Body::empty())
            .unwrap();
        let blocked_list_after_resp = app.clone().oneshot(blocked_list_after_req).await.unwrap();
        assert_eq!(blocked_list_after_resp.status(), StatusCode::OK);
        let blocked_list_after_body = blocked_list_after_resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let blocked_list_after_json: serde_json::Value =
            serde_json::from_slice(&blocked_list_after_body).unwrap();
        assert_eq!(blocked_list_after_json.as_array().map(|v| v.len()), Some(0));

        let alice_profile_after_req = Request::builder()
            .uri("/api/profile/alice@unittest")
            .method("POST")
            .header("Authorization", format!("Bearer {bob_token}"))
            .body(Body::empty())
            .unwrap();
        let alice_profile_after_resp = app.oneshot(alice_profile_after_req).await.unwrap();
        assert_eq!(alice_profile_after_resp.status(), StatusCode::OK);
        let alice_profile_after_body = alice_profile_after_resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let alice_profile_after_json: serde_json::Value =
            serde_json::from_slice(&alice_profile_after_body).unwrap();
        assert_eq!(
            alice_profile_after_json
                .get("isBlocked")
                .and_then(|v| v.as_bool()),
            Some(false)
        );
    }

    #[tokio::test]
    async fn guest_login_roundtrip() {
        let (app, state) = build_router(test_config()).await.expect("build router");
        let app = app.with_state(state);

        let guest_req = Request::builder()
            .uri("/api/guest/login")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"guestId":"guest-a","remember":true}"#))
            .unwrap();
        let guest_resp = app.oneshot(guest_req).await.unwrap();
        assert_eq!(guest_resp.status(), StatusCode::OK);
        let guest_body = guest_resp.into_body().collect().await.unwrap().to_bytes();
        let guest_json: serde_json::Value = serde_json::from_slice(&guest_body).unwrap();
        assert_eq!(
            guest_json.get("email").and_then(|v| v.as_str()),
            Some("guest-a")
        );
        assert!(guest_json.get("token").and_then(|v| v.as_str()).is_some());
    }

    #[tokio::test]
    async fn api_topic_info_not_found_returns_404() {
        let (app, state) = build_router(test_config()).await.expect("build router");
        let app = app.with_state(state);

        let token = register_and_auth(&app, "topic404-user").await;

        let req = Request::builder()
            .uri("/api/topic/info/topic-not-found")
            .method("POST")
            .header("Authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn api_chat_info_not_found_returns_404() {
        let (app, state) = build_router(test_config()).await.expect("build router");
        let app = app.with_state(state);

        let token = register_and_auth(&app, "chat404-user").await;

        let req = Request::builder()
            .uri("/api/chat/info/topic-not-found")
            .method("POST")
            .header("Authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn api_topic_knock_private_topic_returns_401() {
        let (app, state) = build_router(test_config()).await.expect("build router");
        let app = app.with_state(state);

        let owner_token = register_and_auth(&app, "private-owner").await;
        let joiner_token = register_and_auth(&app, "private-joiner").await;

        let create_req = Request::builder()
            .uri("/api/topic/create")
            .method("POST")
            .header("Authorization", format!("Bearer {owner_token}"))
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"name":"private-room","members":["private-owner"],"private":true,"multiple":true}"#,
            ))
            .unwrap();
        let create_resp = app.clone().oneshot(create_req).await.unwrap();
        assert_eq!(create_resp.status(), StatusCode::OK);
        let create_body = create_resp.into_body().collect().await.unwrap().to_bytes();
        let create_json: serde_json::Value = serde_json::from_slice(&create_body).unwrap();
        let topic_id = create_json.get("id").and_then(|v| v.as_str()).unwrap();

        let knock_req = Request::builder()
            .uri(format!("/api/topic/knock/{topic_id}"))
            .method("POST")
            .header("Authorization", format!("Bearer {joiner_token}"))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"message":"join please"}"#))
            .unwrap();
        let knock_resp = app.oneshot(knock_req).await.unwrap();
        assert_eq!(knock_resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn openapi_topic_send_requires_sender_id() {
        let (app, state) = build_router(test_config()).await.expect("build router");
        let app = app.with_state(state);

        let create_req = Request::builder()
            .uri("/open/topic/create/topic-send-validate")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"senderId":"open-admin","members":["open-admin"],"multiple":true}"#,
            ))
            .unwrap();
        let create_resp = app.clone().oneshot(create_req).await.unwrap();
        assert_eq!(create_resp.status(), StatusCode::OK);

        let send_req = Request::builder()
            .uri("/open/topic/send/topic-send-validate")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"type":"chat","chatId":"m-1","message":"hi"}"#,
            ))
            .unwrap();
        let send_resp = app.oneshot(send_req).await.unwrap();
        assert_eq!(send_resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn openapi_user_relation_same_ids_returns_400() {
        let (app, state) = build_router(test_config()).await.expect("build router");
        let app = app.with_state(state);

        let req = Request::builder()
            .uri("/open/user/relation/u1/u1")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"isContact":true}"#))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn openapi_user_auth_missing_user_and_blacklist_empty_ids_return_errors() {
        let (app, state) = build_router(test_config()).await.expect("build router");
        let app = app.with_state(state);

        let auth_req = Request::builder()
            .uri("/open/user/auth/missing-user")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"createWhenNotExist":false}"#))
            .unwrap();
        let auth_resp = app.clone().oneshot(auth_req).await.unwrap();
        assert_eq!(auth_resp.status(), StatusCode::NOT_FOUND);

        let blacklist_add_req = Request::builder()
            .uri("/open/user/blacklist/add/missing-user")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"userIds":[]}"#))
            .unwrap();
        let blacklist_add_resp = app.clone().oneshot(blacklist_add_req).await.unwrap();
        assert_eq!(blacklist_add_resp.status(), StatusCode::BAD_REQUEST);

        let blacklist_remove_req = Request::builder()
            .uri("/open/user/blacklist/remove/missing-user")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"userIds":[]}"#))
            .unwrap();
        let blacklist_remove_resp = app.oneshot(blacklist_remove_req).await.unwrap();
        assert_eq!(blacklist_remove_resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn chat_clear_and_remove_messages_affect_sync_results() {
        let (app, state) = build_router(test_config()).await.expect("build router");
        let app = app.with_state(state);

        let user_token = register_and_auth(&app, "carol").await;

        let topic_req = Request::builder()
            .uri("/api/topic/create/dave")
            .method("POST")
            .header("Authorization", format!("Bearer {user_token}"))
            .body(Body::empty())
            .unwrap();
        let topic_resp = app.clone().oneshot(topic_req).await.unwrap();
        assert_eq!(topic_resp.status(), StatusCode::OK);
        let topic_body = topic_resp.into_body().collect().await.unwrap().to_bytes();
        let topic_json: serde_json::Value = serde_json::from_slice(&topic_body).unwrap();
        let topic_id = topic_json
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap()
            .to_string();

        let send1 = Request::builder()
            .uri(format!("/api/chat/send/{topic_id}"))
            .method("POST")
            .header("Authorization", format!("Bearer {user_token}"))
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"type":"chat","chatId":"m1","message":"one"}"#,
            ))
            .unwrap();
        let resp1 = app.clone().oneshot(send1).await.unwrap();
        assert_eq!(resp1.status(), StatusCode::OK);

        let send2 = Request::builder()
            .uri(format!("/api/chat/send/{topic_id}"))
            .method("POST")
            .header("Authorization", format!("Bearer {user_token}"))
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"type":"chat","chatId":"m2","message":"two"}"#,
            ))
            .unwrap();
        let resp2 = app.clone().oneshot(send2).await.unwrap();
        assert_eq!(resp2.status(), StatusCode::OK);

        let remove_req = Request::builder()
            .uri(format!("/api/chat/remove_messages/{topic_id}"))
            .method("POST")
            .header("Authorization", format!("Bearer {user_token}"))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"ids":["m1"]}"#))
            .unwrap();
        let remove_resp = app.clone().oneshot(remove_req).await.unwrap();
        if remove_resp.status() != StatusCode::OK {
            let body = remove_resp.into_body().collect().await.unwrap().to_bytes();
            panic!(
                "remove_messages failed: {}",
                String::from_utf8(body.to_vec()).unwrap()
            );
        }

        let sync_req = Request::builder()
            .uri(format!("/api/chat/sync/{topic_id}"))
            .method("POST")
            .header("Authorization", format!("Bearer {user_token}"))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"limit":10}"#))
            .unwrap();
        let sync_resp = app.clone().oneshot(sync_req).await.unwrap();
        assert_eq!(sync_resp.status(), StatusCode::OK);
        let sync_body = sync_resp.into_body().collect().await.unwrap().to_bytes();
        let sync_json: serde_json::Value = serde_json::from_slice(&sync_body).unwrap();
        let items = sync_json.get("items").and_then(|v| v.as_array()).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].get("id").and_then(|v| v.as_str()), Some("m2"));
        assert_eq!(
            items[0]
                .get("content")
                .and_then(|v| v.get("type"))
                .and_then(|v| v.as_str()),
            Some("chat")
        );
        assert_eq!(items[1].get("id").and_then(|v| v.as_str()), Some("m1"));
        assert_eq!(
            items[1]
                .get("content")
                .and_then(|v| v.get("type"))
                .and_then(|v| v.as_str()),
            Some("")
        );
        assert_eq!(
            items[1]
                .get("content")
                .and_then(|v| v.get("text"))
                .and_then(|v| v.as_str()),
            Some("")
        );

        let clear_req = Request::builder()
            .uri(format!("/api/chat/clear_messages/{topic_id}"))
            .method("POST")
            .header("Authorization", format!("Bearer {user_token}"))
            .body(Body::empty())
            .unwrap();
        let clear_resp = app.clone().oneshot(clear_req).await.unwrap();
        assert_eq!(clear_resp.status(), StatusCode::OK);

        let sync_after_clear_req = Request::builder()
            .uri(format!("/api/chat/sync/{topic_id}"))
            .method("POST")
            .header("Authorization", format!("Bearer {user_token}"))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"limit":10}"#))
            .unwrap();
        let sync_after_clear_resp = app.oneshot(sync_after_clear_req).await.unwrap();
        assert_eq!(sync_after_clear_resp.status(), StatusCode::OK);
        let body = sync_after_clear_resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(!text.contains("\"id\":\"m2\""));
    }

    #[tokio::test]
    async fn chat_send_with_attendee_creates_dm_topic() {
        let (app, state) = build_router(test_config()).await.expect("build router");
        let app = app.with_state(state);

        let sender_token = register_and_auth(&app, "eve").await;
        let _receiver_token = register_and_auth(&app, "frank").await;

        let send_req = Request::builder()
            .uri("/api/chat/send")
            .method("POST")
            .header("Authorization", format!("Bearer {sender_token}"))
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"type":"chat","chatId":"dm1","attendee":"frank","message":"hello frank"}"#,
            ))
            .unwrap();
        let send_resp = app.clone().oneshot(send_req).await.unwrap();
        assert_eq!(send_resp.status(), StatusCode::OK);
        let send_body = send_resp.into_body().collect().await.unwrap().to_bytes();
        let send_json: serde_json::Value = serde_json::from_slice(&send_body).unwrap();
        let topic_id = send_json
            .get("topicId")
            .and_then(|v| v.as_str())
            .unwrap()
            .to_string();
        assert_eq!(topic_id, "eve:frank");

        let sync_req = Request::builder()
            .uri(format!("/api/chat/sync/{topic_id}"))
            .method("POST")
            .header("Authorization", format!("Bearer {sender_token}"))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"limit":10}"#))
            .unwrap();
        let sync_resp = app.oneshot(sync_req).await.unwrap();
        assert_eq!(sync_resp.status(), StatusCode::OK);
        let sync_body = sync_resp.into_body().collect().await.unwrap().to_bytes();
        let sync_json: serde_json::Value = serde_json::from_slice(&sync_body).unwrap();
        let items = sync_json.get("items").and_then(|v| v.as_array()).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].get("id").and_then(|v| v.as_str()), Some("dm1"));
        assert_eq!(
            items[0]
                .get("content")
                .and_then(|v| v.get("text"))
                .and_then(|v| v.as_str()),
            Some("hello frank")
        );
    }

    #[tokio::test]
    async fn topic_knock_notice_and_admin_flow() {
        let (app, state) = build_router(test_config()).await.expect("build router");
        let app = app.with_state(state);

        let owner_token = register_and_auth(&app, "owner1").await;
        let joiner_token = register_and_auth(&app, "joiner1").await;

        let create_req = Request::builder()
            .uri("/api/topic/create")
            .method("POST")
            .header("Authorization", format!("Bearer {owner_token}"))
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"name":"g1","members":["owner1"],"knockNeedVerify":true}"#,
            ))
            .unwrap();
        let create_resp = app.clone().oneshot(create_req).await.unwrap();
        assert_eq!(create_resp.status(), StatusCode::OK);
        let create_body = create_resp.into_body().collect().await.unwrap().to_bytes();
        let create_json: serde_json::Value = serde_json::from_slice(&create_body).unwrap();
        let topic_id = create_json
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap()
            .to_string();

        let knock_req = Request::builder()
            .uri(format!("/api/topic/knock/{topic_id}"))
            .method("POST")
            .header("Authorization", format!("Bearer {joiner_token}"))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"message":"let me in","source":"mobile"}"#))
            .unwrap();
        let knock_resp = app.clone().oneshot(knock_req).await.unwrap();
        assert_eq!(knock_resp.status(), StatusCode::OK);

        let list_knocks_req = Request::builder()
            .uri(format!("/api/topic/admin/list_knock/{topic_id}"))
            .method("POST")
            .header("Authorization", format!("Bearer {owner_token}"))
            .body(Body::empty())
            .unwrap();
        let list_knocks_resp = app.clone().oneshot(list_knocks_req).await.unwrap();
        if list_knocks_resp.status() != StatusCode::OK {
            let body = list_knocks_resp
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes();
            panic!(
                "list_knock failed: {}",
                String::from_utf8(body.to_vec()).unwrap()
            );
        }
        let knocks_body = list_knocks_resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let knocks_text = String::from_utf8(knocks_body.to_vec()).unwrap();
        assert!(knocks_text.contains("joiner1"));

        let accept_req = Request::builder()
            .uri(format!("/api/topic/admin/knock/accept/{topic_id}/joiner1"))
            .method("POST")
            .header("Authorization", format!("Bearer {owner_token}"))
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .unwrap();
        let accept_resp = app.clone().oneshot(accept_req).await.unwrap();
        assert_eq!(accept_resp.status(), StatusCode::OK);

        let notice_req = Request::builder()
            .uri(format!("/api/topic/admin/notice/{topic_id}"))
            .method("POST")
            .header("Authorization", format!("Bearer {owner_token}"))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"text":"welcome"}"#))
            .unwrap();
        let notice_resp = app.clone().oneshot(notice_req).await.unwrap();
        assert_eq!(notice_resp.status(), StatusCode::OK);

        let info_req = Request::builder()
            .uri(format!("/api/topic/info/{topic_id}"))
            .method("POST")
            .header("Authorization", format!("Bearer {joiner_token}"))
            .body(Body::empty())
            .unwrap();
        let info_resp = app.oneshot(info_req).await.unwrap();
        assert_eq!(info_resp.status(), StatusCode::OK);
        let info_body = info_resp.into_body().collect().await.unwrap().to_bytes();
        let info_text = String::from_utf8(info_body.to_vec()).unwrap();
        assert!(info_text.contains("welcome"));
    }

    #[tokio::test]
    async fn websocket_connect_and_chat_ack_flow() {
        let config = test_config();
        let (app, state) = build_router(config).await.expect("build router");

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app.with_state(state)).await.unwrap();
        });

        let endpoint = format!("http://{}", addr);
        let client = reqwest::Client::new();

        let _ = client
            .post(format!("{endpoint}/open/user/register/ws-alice"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();
        let _ = client
            .post(format!("{endpoint}/open/user/register/ws-bob"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();

        let alice_auth: serde_json::Value = client
            .post(format!("{endpoint}/open/user/auth/ws-alice"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let bob_auth: serde_json::Value = client
            .post(format!("{endpoint}/open/user/auth/ws-bob"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

        let alice_token = alice_auth
            .get("authToken")
            .and_then(|v| v.as_str())
            .unwrap();
        let bob_token = bob_auth.get("authToken").and_then(|v| v.as_str()).unwrap();

        let mut alice_req = format!("ws://{}/api/connect?device=test-alice", addr)
            .into_client_request()
            .unwrap();
        alice_req.headers_mut().insert(
            "Authorization",
            format!("Bearer {alice_token}").parse().unwrap(),
        );
        let (mut alice_ws, _) = tokio_tungstenite::connect_async(alice_req).await.unwrap();

        let mut bob_req = format!("ws://{}/api/connect?device=test-bob", addr)
            .into_client_request()
            .unwrap();
        bob_req.headers_mut().insert(
            "Authorization",
            format!("Bearer {bob_token}").parse().unwrap(),
        );
        let (mut bob_ws, _) = tokio_tungstenite::connect_async(bob_req).await.unwrap();

        bob_ws
            .send(tokio_tungstenite::tungstenite::Message::Text(
                serde_json::json!({
                    "type": "chat",
                    "chatId": "ws-msg-1",
                    "attendee": "ws-alice",
                    "message": "hello from ws"
                })
                .to_string(),
            ))
            .await
            .unwrap();

        let alice_msg = alice_ws.next().await.unwrap().unwrap();
        let alice_text = alice_msg.into_text().unwrap();
        let alice_json: serde_json::Value = serde_json::from_str(&alice_text).unwrap();
        assert_eq!(
            alice_json.get("type").and_then(|v| v.as_str()),
            Some("chat")
        );
        assert_eq!(
            alice_json.get("chatId").and_then(|v| v.as_str()),
            Some("ws-msg-1")
        );
        assert_eq!(
            alice_json
                .get("content")
                .and_then(|v| v.get("text"))
                .and_then(|v| v.as_str()),
            Some("hello from ws")
        );

        let bob_event = bob_ws.next().await.unwrap().unwrap();
        let bob_event_text = bob_event.into_text().unwrap();
        let bob_event_json: serde_json::Value = serde_json::from_str(&bob_event_text).unwrap();
        assert_eq!(
            bob_event_json.get("type").and_then(|v| v.as_str()),
            Some("chat")
        );

        let bob_ack = bob_ws.next().await.unwrap().unwrap();
        let bob_ack_text = bob_ack.into_text().unwrap();
        let bob_ack_json: serde_json::Value = serde_json::from_str(&bob_ack_text).unwrap();
        assert_eq!(
            bob_ack_json.get("type").and_then(|v| v.as_str()),
            Some("resp")
        );
        assert_eq!(
            bob_ack_json.get("chatId").and_then(|v| v.as_str()),
            Some("ws-msg-1")
        );
        assert_eq!(bob_ack_json.get("code").and_then(|v| v.as_i64()), Some(200));

        server.abort();
    }

    #[tokio::test]
    async fn websocket_recall_flow_updates_sync_and_emits_recall_event() {
        let config = test_config();
        let (app, state) = build_router(config).await.expect("build router");

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app.with_state(state)).await.unwrap();
        });

        let endpoint = format!("http://{}", addr);
        let client = reqwest::Client::new();

        let _ = client
            .post(format!("{endpoint}/open/user/register/recall-alice"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();
        let _ = client
            .post(format!("{endpoint}/open/user/register/recall-bob"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();

        let alice_auth: serde_json::Value = client
            .post(format!("{endpoint}/open/user/auth/recall-alice"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let bob_auth: serde_json::Value = client
            .post(format!("{endpoint}/open/user/auth/recall-bob"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

        let alice_token = alice_auth
            .get("authToken")
            .and_then(|v| v.as_str())
            .unwrap();
        let bob_token = bob_auth.get("authToken").and_then(|v| v.as_str()).unwrap();

        let mut alice_req = format!("ws://{}/api/connect?device=recall-alice", addr)
            .into_client_request()
            .unwrap();
        alice_req.headers_mut().insert(
            "Authorization",
            format!("Bearer {alice_token}").parse().unwrap(),
        );
        let (mut alice_ws, _) = tokio_tungstenite::connect_async(alice_req).await.unwrap();

        let mut bob_req = format!("ws://{}/api/connect?device=recall-bob", addr)
            .into_client_request()
            .unwrap();
        bob_req.headers_mut().insert(
            "Authorization",
            format!("Bearer {bob_token}").parse().unwrap(),
        );
        let (mut bob_ws, _) = tokio_tungstenite::connect_async(bob_req).await.unwrap();

        bob_ws
            .send(tokio_tungstenite::tungstenite::Message::Text(
                serde_json::json!({
                    "type": "chat",
                    "chatId": "recall-msg-1",
                    "attendee": "recall-alice",
                    "message": "hello recall"
                })
                .to_string(),
            ))
            .await
            .unwrap();

        let _ = alice_ws.next().await.unwrap().unwrap();
        let _ = bob_ws.next().await.unwrap().unwrap();
        let _ = bob_ws.next().await.unwrap().unwrap();

        bob_ws
            .send(tokio_tungstenite::tungstenite::Message::Text(
                serde_json::json!({
                    "type": "chat",
                    "chatId": "recall-event-1",
                    "topicId": "recall-alice:recall-bob",
                    "content": {
                        "type": "recall",
                        "text": "recall-msg-1"
                    }
                })
                .to_string(),
            ))
            .await
            .unwrap();

        let alice_recall = alice_ws.next().await.unwrap().unwrap().into_text().unwrap();
        let alice_recall_json: serde_json::Value = serde_json::from_str(&alice_recall).unwrap();
        assert_eq!(
            alice_recall_json.get("type").and_then(|v| v.as_str()),
            Some("chat")
        );
        assert_eq!(
            alice_recall_json.get("chatId").and_then(|v| v.as_str()),
            Some("recall-event-1")
        );
        assert_eq!(
            alice_recall_json
                .get("content")
                .and_then(|v| v.get("type"))
                .and_then(|v| v.as_str()),
            Some("recall")
        );
        assert_eq!(
            alice_recall_json
                .get("content")
                .and_then(|v| v.get("text"))
                .and_then(|v| v.as_str()),
            Some("recall-msg-1")
        );

        let bob_recall = bob_ws.next().await.unwrap().unwrap().into_text().unwrap();
        let bob_recall_json: serde_json::Value = serde_json::from_str(&bob_recall).unwrap();
        assert_eq!(
            bob_recall_json.get("type").and_then(|v| v.as_str()),
            Some("chat")
        );

        let bob_ack = bob_ws.next().await.unwrap().unwrap().into_text().unwrap();
        let bob_ack_json: serde_json::Value = serde_json::from_str(&bob_ack).unwrap();
        assert_eq!(
            bob_ack_json.get("type").and_then(|v| v.as_str()),
            Some("resp")
        );
        assert_eq!(
            bob_ack_json.get("chatId").and_then(|v| v.as_str()),
            Some("recall-event-1")
        );

        let sync_resp: serde_json::Value = client
            .post(format!("{endpoint}/api/chat/sync/recall-alice:recall-bob"))
            .bearer_auth(bob_token)
            .json(&serde_json::json!({"limit": 10}))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let items = sync_resp.get("items").and_then(|v| v.as_array()).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(
            items[0].get("id").and_then(|v| v.as_str()),
            Some("recall-event-1")
        );
        assert_eq!(
            items[1].get("id").and_then(|v| v.as_str()),
            Some("recall-msg-1")
        );
        assert_eq!(items[1].get("recall").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(
            items[1]
                .get("content")
                .and_then(|v| v.get("type"))
                .and_then(|v| v.as_str()),
            Some("recalled")
        );

        server.abort();
    }

    #[tokio::test]
    async fn websocket_reconnect_and_multi_device_fanout() {
        let config = test_config();
        let (app, state) = build_router(config).await.expect("build router");

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app.with_state(state)).await.unwrap();
        });

        let endpoint = format!("http://{}", addr);
        let client = reqwest::Client::new();

        let _ = client
            .post(format!("{endpoint}/open/user/register/reconnect-alice"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();
        let _ = client
            .post(format!("{endpoint}/open/user/register/reconnect-bob"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();

        let alice_auth: serde_json::Value = client
            .post(format!("{endpoint}/open/user/auth/reconnect-alice"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let bob_auth: serde_json::Value = client
            .post(format!("{endpoint}/open/user/auth/reconnect-bob"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

        let alice_token = alice_auth
            .get("authToken")
            .and_then(|v| v.as_str())
            .unwrap();
        let bob_token = bob_auth.get("authToken").and_then(|v| v.as_str()).unwrap();

        let mut alice_phone_req = format!("ws://{}/api/connect?device=phone", addr)
            .into_client_request()
            .unwrap();
        alice_phone_req.headers_mut().insert(
            "Authorization",
            format!("Bearer {alice_token}").parse().unwrap(),
        );
        let (mut alice_phone_ws, _) = tokio_tungstenite::connect_async(alice_phone_req)
            .await
            .unwrap();

        let mut alice_laptop_req = format!("ws://{}/api/connect?device=laptop", addr)
            .into_client_request()
            .unwrap();
        alice_laptop_req.headers_mut().insert(
            "Authorization",
            format!("Bearer {alice_token}").parse().unwrap(),
        );
        let (mut alice_laptop_ws, _) = tokio_tungstenite::connect_async(alice_laptop_req)
            .await
            .unwrap();

        let mut bob_req = format!("ws://{}/api/connect?device=bob", addr)
            .into_client_request()
            .unwrap();
        bob_req.headers_mut().insert(
            "Authorization",
            format!("Bearer {bob_token}").parse().unwrap(),
        );
        let (mut bob_ws, _) = tokio_tungstenite::connect_async(bob_req).await.unwrap();

        bob_ws
            .send(tokio_tungstenite::tungstenite::Message::Text(
                serde_json::json!({
                    "type": "chat",
                    "chatId": "multi-1",
                    "attendee": "reconnect-alice",
                    "message": "hello both devices"
                })
                .to_string(),
            ))
            .await
            .unwrap();

        let alice_phone_msg =
            tokio::time::timeout(std::time::Duration::from_secs(3), alice_phone_ws.next())
                .await
                .expect("alice phone recv timeout")
                .expect("alice phone stream ended")
                .expect("alice phone ws error")
                .into_text()
                .unwrap();
        let alice_phone_json: serde_json::Value = serde_json::from_str(&alice_phone_msg).unwrap();
        assert_eq!(
            alice_phone_json.get("type").and_then(|v| v.as_str()),
            Some("chat")
        );
        assert_eq!(
            alice_phone_json.get("chatId").and_then(|v| v.as_str()),
            Some("multi-1")
        );

        let alice_laptop_msg =
            tokio::time::timeout(std::time::Duration::from_secs(3), alice_laptop_ws.next())
                .await
                .expect("alice laptop recv timeout")
                .expect("alice laptop stream ended")
                .expect("alice laptop ws error")
                .into_text()
                .unwrap();
        let alice_laptop_json: serde_json::Value = serde_json::from_str(&alice_laptop_msg).unwrap();
        assert_eq!(
            alice_laptop_json.get("type").and_then(|v| v.as_str()),
            Some("chat")
        );
        assert_eq!(
            alice_laptop_json.get("chatId").and_then(|v| v.as_str()),
            Some("multi-1")
        );

        let mut saw_bob_chat = false;
        let mut saw_bob_ack = false;
        for _ in 0..4 {
            let msg = tokio::time::timeout(std::time::Duration::from_secs(3), bob_ws.next())
                .await
                .expect("bob recv timeout")
                .expect("bob stream ended")
                .expect("bob ws error")
                .into_text()
                .unwrap();
            let value: serde_json::Value = serde_json::from_str(&msg).unwrap();
            match value.get("type").and_then(|v| v.as_str()) {
                Some("chat") => saw_bob_chat = true,
                Some("resp") => {
                    if value.get("chatId").and_then(|v| v.as_str()) == Some("multi-1") {
                        saw_bob_ack = true;
                    }
                }
                _ => {}
            }
            if saw_bob_chat && saw_bob_ack {
                break;
            }
        }
        assert!(saw_bob_chat && saw_bob_ack);

        let _ = alice_phone_ws.close(None).await;
        let _ = alice_laptop_ws.close(None).await;
        drop(alice_phone_ws);
        drop(alice_laptop_ws);

        let mut final_online = true;
        for _ in 0..40 {
            let online_resp: serde_json::Value = client
                .post(format!("{endpoint}/open/user/online/reconnect-alice"))
                .bearer_auth("test-token")
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
            final_online = online_resp
                .get("online")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if !final_online {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        assert!(
            !final_online,
            "alice should become offline after both sockets close"
        );

        let mut alice_reconnect_req = format!("ws://{}/api/connect?device=phone", addr)
            .into_client_request()
            .unwrap();
        alice_reconnect_req.headers_mut().insert(
            "Authorization",
            format!("Bearer {alice_token}").parse().unwrap(),
        );
        let (mut alice_reconnect_ws, _) = tokio_tungstenite::connect_async(alice_reconnect_req)
            .await
            .unwrap();

        let online_after_reconnect: serde_json::Value = client
            .post(format!("{endpoint}/open/user/online/reconnect-alice"))
            .bearer_auth("test-token")
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(
            online_after_reconnect
                .get("online")
                .and_then(|v| v.as_bool()),
            Some(true)
        );
        assert!(online_after_reconnect
            .get("devices")
            .and_then(|v| v.as_array())
            .is_some_and(|arr| arr.iter().any(|v| v.as_str() == Some("phone"))));

        bob_ws
            .send(tokio_tungstenite::tungstenite::Message::Text(
                serde_json::json!({
                    "type": "chat",
                    "chatId": "multi-2",
                    "attendee": "reconnect-alice",
                    "message": "hello after reconnect"
                })
                .to_string(),
            ))
            .await
            .unwrap();

        let alice_reconnect_msg =
            tokio::time::timeout(std::time::Duration::from_secs(3), alice_reconnect_ws.next())
                .await
                .expect("alice reconnect recv timeout")
                .expect("alice reconnect stream ended")
                .expect("alice reconnect ws error")
                .into_text()
                .unwrap();
        let alice_reconnect_json: serde_json::Value =
            serde_json::from_str(&alice_reconnect_msg).unwrap();
        assert_eq!(
            alice_reconnect_json.get("type").and_then(|v| v.as_str()),
            Some("chat")
        );
        assert_eq!(
            alice_reconnect_json.get("chatId").and_then(|v| v.as_str()),
            Some("multi-2")
        );

        let mut saw_bob_chat2 = false;
        let mut saw_bob_ack2 = false;
        for _ in 0..4 {
            let msg = tokio::time::timeout(std::time::Duration::from_secs(3), bob_ws.next())
                .await
                .expect("bob recv timeout")
                .expect("bob stream ended")
                .expect("bob ws error")
                .into_text()
                .unwrap();
            let value: serde_json::Value = serde_json::from_str(&msg).unwrap();
            match value.get("type").and_then(|v| v.as_str()) {
                Some("chat") => saw_bob_chat2 = true,
                Some("resp") => {
                    if value.get("chatId").and_then(|v| v.as_str()) == Some("multi-2") {
                        saw_bob_ack2 = true;
                    }
                }
                _ => {}
            }
            if saw_bob_chat2 && saw_bob_ack2 {
                break;
            }
        }
        assert!(saw_bob_chat2 && saw_bob_ack2);

        let _ = alice_reconnect_ws.close(None).await;
        let _ = bob_ws.close(None).await;
        server.abort();
    }

    #[tokio::test]
    async fn websocket_reconnect_storm_preserves_sync_order() {
        let config = test_config();
        let (app, state) = build_router(config).await.expect("build router");

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app.with_state(state)).await.unwrap();
        });

        let endpoint = format!("http://{}", addr);
        let client = reqwest::Client::new();

        let _ = client
            .post(format!("{endpoint}/open/user/register/storm-a"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();
        let _ = client
            .post(format!("{endpoint}/open/user/register/storm-b"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();

        let alice_auth: serde_json::Value = client
            .post(format!("{endpoint}/open/user/auth/storm-a"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let bob_auth: serde_json::Value = client
            .post(format!("{endpoint}/open/user/auth/storm-b"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let alice_token = alice_auth
            .get("authToken")
            .and_then(|v| v.as_str())
            .unwrap();
        let bob_token = bob_auth.get("authToken").and_then(|v| v.as_str()).unwrap();

        let mut bob_req = format!("ws://{}/api/connect?device=storm-b", addr)
            .into_client_request()
            .unwrap();
        bob_req.headers_mut().insert(
            "Authorization",
            format!("Bearer {bob_token}").parse().unwrap(),
        );
        let (mut bob_ws, _) = tokio_tungstenite::connect_async(bob_req).await.unwrap();

        let mut alice_req = format!("ws://{}/api/connect?device=storm-a-1", addr)
            .into_client_request()
            .unwrap();
        alice_req.headers_mut().insert(
            "Authorization",
            format!("Bearer {alice_token}").parse().unwrap(),
        );
        let (mut alice_ws, _) = tokio_tungstenite::connect_async(alice_req).await.unwrap();

        bob_ws
            .send(tokio_tungstenite::tungstenite::Message::Text(
                serde_json::json!({
                    "type": "chat",
                    "chatId": "storm-1",
                    "attendee": "storm-a",
                    "message": "first"
                })
                .to_string(),
            ))
            .await
            .unwrap();

        let msg_1 = tokio::time::timeout(std::time::Duration::from_secs(3), alice_ws.next())
            .await
            .expect("alice first recv timeout")
            .expect("alice first stream ended")
            .expect("alice first ws error")
            .into_text()
            .unwrap();
        let msg_1_json: serde_json::Value = serde_json::from_str(&msg_1).unwrap();
        assert_eq!(
            msg_1_json.get("chatId").and_then(|v| v.as_str()),
            Some("storm-1")
        );

        let mut saw_storm_1_ack = false;
        for _ in 0..4 {
            let msg = tokio::time::timeout(std::time::Duration::from_secs(3), bob_ws.next())
                .await
                .expect("bob first recv timeout")
                .expect("bob first stream ended")
                .expect("bob first ws error")
                .into_text()
                .unwrap();
            let value: serde_json::Value = serde_json::from_str(&msg).unwrap();
            if value.get("type").and_then(|v| v.as_str()) == Some("resp")
                && value.get("chatId").and_then(|v| v.as_str()) == Some("storm-1")
            {
                saw_storm_1_ack = true;
                break;
            }
        }
        assert!(saw_storm_1_ack);

        let _ = alice_ws.close(None).await;
        drop(alice_ws);

        bob_ws
            .send(tokio_tungstenite::tungstenite::Message::Text(
                serde_json::json!({
                    "type": "chat",
                    "chatId": "storm-2",
                    "attendee": "storm-a",
                    "message": "second"
                })
                .to_string(),
            ))
            .await
            .unwrap();

        let mut saw_storm_2_ack = false;
        for _ in 0..4 {
            let msg = tokio::time::timeout(std::time::Duration::from_secs(3), bob_ws.next())
                .await
                .expect("bob second recv timeout")
                .expect("bob second stream ended")
                .expect("bob second ws error")
                .into_text()
                .unwrap();
            let value: serde_json::Value = serde_json::from_str(&msg).unwrap();
            if value.get("type").and_then(|v| v.as_str()) == Some("resp")
                && value.get("chatId").and_then(|v| v.as_str()) == Some("storm-2")
            {
                saw_storm_2_ack = true;
                break;
            }
        }
        assert!(saw_storm_2_ack);

        let mut alice_reconnect_req = format!("ws://{}/api/connect?device=storm-a-2", addr)
            .into_client_request()
            .unwrap();
        alice_reconnect_req.headers_mut().insert(
            "Authorization",
            format!("Bearer {alice_token}").parse().unwrap(),
        );
        let (mut alice_reconnect_ws, _) = tokio_tungstenite::connect_async(alice_reconnect_req)
            .await
            .unwrap();

        bob_ws
            .send(tokio_tungstenite::tungstenite::Message::Text(
                serde_json::json!({
                    "type": "chat",
                    "chatId": "storm-3",
                    "attendee": "storm-a",
                    "message": "third"
                })
                .to_string(),
            ))
            .await
            .unwrap();

        let msg_3 =
            tokio::time::timeout(std::time::Duration::from_secs(3), alice_reconnect_ws.next())
                .await
                .expect("alice reconnect recv timeout")
                .expect("alice reconnect stream ended")
                .expect("alice reconnect ws error")
                .into_text()
                .unwrap();
        let msg_3_json: serde_json::Value = serde_json::from_str(&msg_3).unwrap();
        assert_eq!(
            msg_3_json.get("chatId").and_then(|v| v.as_str()),
            Some("storm-3")
        );

        let mut saw_storm_3_ack = false;
        for _ in 0..4 {
            let msg = tokio::time::timeout(std::time::Duration::from_secs(3), bob_ws.next())
                .await
                .expect("bob third recv timeout")
                .expect("bob third stream ended")
                .expect("bob third ws error")
                .into_text()
                .unwrap();
            let value: serde_json::Value = serde_json::from_str(&msg).unwrap();
            if value.get("type").and_then(|v| v.as_str()) == Some("resp")
                && value.get("chatId").and_then(|v| v.as_str()) == Some("storm-3")
            {
                saw_storm_3_ack = true;
                break;
            }
        }
        assert!(saw_storm_3_ack);

        let sync_resp: serde_json::Value = client
            .post(format!("{endpoint}/api/chat/sync/storm-a:storm-b"))
            .bearer_auth(alice_token)
            .json(&serde_json::json!({"limit": 10}))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

        let items = sync_resp.get("items").and_then(|v| v.as_array()).unwrap();
        let storm_items: Vec<&serde_json::Value> = items
            .iter()
            .filter(|item| {
                item.get("id")
                    .and_then(|v| v.as_str())
                    .is_some_and(|id| id.starts_with("storm-"))
            })
            .collect();
        assert_eq!(storm_items.len(), 3);
        assert_eq!(
            storm_items[0].get("id").and_then(|v| v.as_str()),
            Some("storm-3")
        );
        assert_eq!(
            storm_items[1].get("id").and_then(|v| v.as_str()),
            Some("storm-2")
        );
        assert_eq!(
            storm_items[2].get("id").and_then(|v| v.as_str()),
            Some("storm-1")
        );

        let _ = alice_reconnect_ws.close(None).await;
        let _ = bob_ws.close(None).await;
        server.abort();
    }

    #[tokio::test]
    async fn websocket_ping_read_typing_and_rate_limit_match_go_behavior() {
        let mut config = test_config();
        config.ws_per_user_limit = 1;
        config.ws_typing_interval_ms = 1000;
        let (app, state) = build_router(config).await.expect("build router");

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app.with_state(state)).await.unwrap();
        });

        let endpoint = format!("http://{}", addr);
        let client = reqwest::Client::new();

        let _ = client
            .post(format!("{endpoint}/open/user/register/ws-limit-a"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();
        let _ = client
            .post(format!("{endpoint}/open/user/register/ws-limit-b"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();
        let forbid_topic_resp = client
            .post(format!("{endpoint}/open/topic/create/ws-read-forbid"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({
                "senderId": "ws-limit-a",
                "members": ["ws-limit-a"],
                "multiple": true
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(forbid_topic_resp.status(), reqwest::StatusCode::OK);

        let alice_auth: serde_json::Value = client
            .post(format!("{endpoint}/open/user/auth/ws-limit-a"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let bob_auth: serde_json::Value = client
            .post(format!("{endpoint}/open/user/auth/ws-limit-b"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let alice_token = alice_auth
            .get("authToken")
            .and_then(|v| v.as_str())
            .unwrap();
        let bob_token = bob_auth.get("authToken").and_then(|v| v.as_str()).unwrap();

        let mut alice_req = format!("ws://{}/api/connect?device=alice", addr)
            .into_client_request()
            .unwrap();
        alice_req.headers_mut().insert(
            "Authorization",
            format!("Bearer {alice_token}").parse().unwrap(),
        );
        let (mut alice_ws, _) = tokio_tungstenite::connect_async(alice_req).await.unwrap();

        let mut bob_req = format!("ws://{}/api/connect?device=bob", addr)
            .into_client_request()
            .unwrap();
        bob_req.headers_mut().insert(
            "Authorization",
            format!("Bearer {bob_token}").parse().unwrap(),
        );
        let (mut bob_ws, _) = tokio_tungstenite::connect_async(bob_req).await.unwrap();

        bob_ws
            .send(tokio_tungstenite::tungstenite::Message::Text(
                serde_json::json!({
                    "type": "ping",
                    "chatId": "ping-1",
                    "message": "hello.png",
                    "content": { "type": "image", "text": "hello.png" }
                })
                .to_string(),
            ))
            .await
            .unwrap();
        let ping_ack = bob_ws.next().await.unwrap().unwrap().into_text().unwrap();
        let ping_ack_json: serde_json::Value = serde_json::from_str(&ping_ack).unwrap();
        assert_eq!(
            ping_ack_json.get("type").and_then(|v| v.as_str()),
            Some("resp")
        );
        assert_eq!(
            ping_ack_json.get("chatId").and_then(|v| v.as_str()),
            Some("ping-1")
        );
        assert_eq!(
            ping_ack_json.get("code").and_then(|v| v.as_i64()),
            Some(200)
        );
        assert_eq!(
            ping_ack_json.get("message").and_then(|v| v.as_str()),
            Some("hello.png")
        );

        bob_ws
            .send(tokio_tungstenite::tungstenite::Message::Text(
                serde_json::json!({
                    "type": "chat",
                    "chatId": "limit-ok-1",
                    "attendee": "ws-limit-a",
                    "message": "hello one"
                })
                .to_string(),
            ))
            .await
            .unwrap();
        let alice_chat = alice_ws.next().await.unwrap().unwrap().into_text().unwrap();
        let alice_chat_json: serde_json::Value = serde_json::from_str(&alice_chat).unwrap();
        let topic_id = alice_chat_json
            .get("topicId")
            .and_then(|v| v.as_str())
            .unwrap()
            .to_string();
        let bob_echo = bob_ws.next().await.unwrap().unwrap().into_text().unwrap();
        let bob_echo_json: serde_json::Value = serde_json::from_str(&bob_echo).unwrap();
        assert_eq!(
            bob_echo_json.get("type").and_then(|v| v.as_str()),
            Some("chat")
        );
        let bob_ok_ack = bob_ws.next().await.unwrap().unwrap().into_text().unwrap();
        let bob_ok_ack_json: serde_json::Value = serde_json::from_str(&bob_ok_ack).unwrap();
        assert_eq!(
            bob_ok_ack_json.get("type").and_then(|v| v.as_str()),
            Some("resp")
        );
        assert_eq!(
            bob_ok_ack_json.get("code").and_then(|v| v.as_i64()),
            Some(200)
        );

        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

        bob_ws
            .send(tokio_tungstenite::tungstenite::Message::Text(
                serde_json::json!({
                    "type": "chat",
                    "chatId": "limit-bad-2",
                    "topicId": "not_exist_topic",
                    "content": { "type": "image", "text": "hello.png" }
                })
                .to_string(),
            ))
            .await
            .unwrap();
        let bad_ack = bob_ws.next().await.unwrap().unwrap().into_text().unwrap();
        let bad_ack_json: serde_json::Value = serde_json::from_str(&bad_ack).unwrap();
        assert_eq!(
            bad_ack_json.get("type").and_then(|v| v.as_str()),
            Some("resp")
        );
        assert_eq!(bad_ack_json.get("code").and_then(|v| v.as_i64()), Some(404));

        bob_ws
            .send(tokio_tungstenite::tungstenite::Message::Text(
                serde_json::json!({
                    "type": "chat",
                    "chatId": "limit-hit-3",
                    "topicId": "not_exist_topic",
                    "content": { "type": "image", "text": "hello.png" }
                })
                .to_string(),
            ))
            .await
            .unwrap();
        let too_many_ack = bob_ws.next().await.unwrap().unwrap().into_text().unwrap();
        let too_many_ack_json: serde_json::Value = serde_json::from_str(&too_many_ack).unwrap();
        assert_eq!(
            too_many_ack_json.get("type").and_then(|v| v.as_str()),
            Some("resp")
        );
        assert_eq!(
            too_many_ack_json.get("code").and_then(|v| v.as_i64()),
            Some(429)
        );

        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

        bob_ws
            .send(tokio_tungstenite::tungstenite::Message::Text(
                serde_json::json!({
                    "type": "typing",
                    "chatId": "typing-1",
                    "topicId": topic_id,
                    "attendee": "ws-limit-a"
                })
                .to_string(),
            ))
            .await
            .unwrap();
        let alice_typing = alice_ws.next().await.unwrap().unwrap().into_text().unwrap();
        let alice_typing_json: serde_json::Value = serde_json::from_str(&alice_typing).unwrap();
        assert_eq!(
            alice_typing_json.get("type").and_then(|v| v.as_str()),
            Some("typing")
        );
        let typing_ack = bob_ws.next().await.unwrap().unwrap().into_text().unwrap();
        let typing_ack_json: serde_json::Value = serde_json::from_str(&typing_ack).unwrap();
        assert_eq!(
            typing_ack_json.get("type").and_then(|v| v.as_str()),
            Some("resp")
        );
        assert_eq!(
            typing_ack_json.get("code").and_then(|v| v.as_i64()),
            Some(200)
        );

        bob_ws
            .send(tokio_tungstenite::tungstenite::Message::Text(
                serde_json::json!({
                    "type": "read",
                    "chatId": "read-ok-1",
                    "topicId": topic_id,
                    "seq": 1
                })
                .to_string(),
            ))
            .await
            .unwrap();
        let alice_read = alice_ws.next().await.unwrap().unwrap().into_text().unwrap();
        let alice_read_json: serde_json::Value = serde_json::from_str(&alice_read).unwrap();
        assert_eq!(
            alice_read_json.get("type").and_then(|v| v.as_str()),
            Some("read")
        );
        let read_ack = bob_ws.next().await.unwrap().unwrap().into_text().unwrap();
        let read_ack_json: serde_json::Value = serde_json::from_str(&read_ack).unwrap();
        assert_eq!(
            read_ack_json.get("type").and_then(|v| v.as_str()),
            Some("resp")
        );
        assert_eq!(
            read_ack_json.get("code").and_then(|v| v.as_i64()),
            Some(200)
        );

        bob_ws
            .send(tokio_tungstenite::tungstenite::Message::Text(
                serde_json::json!({
                    "type": "read",
                    "chatId": "read-miss-2",
                    "topicId": "not exist topic",
                    "seq": 1
                })
                .to_string(),
            ))
            .await
            .unwrap();
        let read_missing_ack = bob_ws.next().await.unwrap().unwrap().into_text().unwrap();
        let read_missing_json: serde_json::Value = serde_json::from_str(&read_missing_ack).unwrap();
        assert_eq!(
            read_missing_json.get("code").and_then(|v| v.as_i64()),
            Some(404)
        );

        bob_ws
            .send(tokio_tungstenite::tungstenite::Message::Text(
                serde_json::json!({
                    "type": "read",
                    "chatId": "read-forbid-3",
                    "topicId": "ws-read-forbid",
                    "seq": 1
                })
                .to_string(),
            ))
            .await
            .unwrap();
        let read_forbid_ack = bob_ws.next().await.unwrap().unwrap().into_text().unwrap();
        let read_forbid_json: serde_json::Value = serde_json::from_str(&read_forbid_ack).unwrap();
        assert_eq!(
            read_forbid_json.get("code").and_then(|v| v.as_i64()),
            Some(403)
        );

        let _ = alice_ws.close(None).await;
        let _ = bob_ws.close(None).await;
        server.abort();
    }

    #[tokio::test]
    async fn topic_chat_webhook_event_is_delivered() {
        use std::sync::{Arc, Mutex};

        let hook_payloads: Arc<Mutex<Vec<serde_json::Value>>> = Arc::new(Mutex::new(Vec::new()));
        let hook_payloads_cloned = hook_payloads.clone();

        let hook_app = axum::Router::new().route(
            "/hook",
            axum::routing::post(move |body: String| {
                let hook_payloads = hook_payloads_cloned.clone();
                async move {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&body) {
                        hook_payloads.lock().unwrap().push(value);
                    }
                    axum::http::StatusCode::OK
                }
            }),
        );

        let hook_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let hook_addr = hook_listener.local_addr().unwrap();
        let hook_server = tokio::spawn(async move {
            axum::serve(hook_listener, hook_app).await.unwrap();
        });

        let config = test_config();
        let (app, state) = build_router(config).await.expect("build router");

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app.with_state(state)).await.unwrap();
        });

        let endpoint = format!("http://{}", addr);
        let client = reqwest::Client::new();

        let webhook_url = format!("http://{}/hook", hook_addr);
        let topic_resp: serde_json::Value = client
            .post(format!("{endpoint}/open/topic/create/topic-webhook-1"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({
                "senderId": "hook-alice",
                "members": ["hook-alice", "hook-bob"],
                "multiple": true,
                "webhooks": [webhook_url]
            }))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let topic_id = topic_resp.get("id").and_then(|v| v.as_str()).unwrap();

        let send_resp = client
            .post(format!("{endpoint}/open/topic/send/{topic_id}"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({
                "senderId": "hook-alice",
                "type": "chat",
                "chatId": "hook-chat-1",
                "message": "hello webhook"
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(send_resp.status(), reqwest::StatusCode::OK);

        for _ in 0..50 {
            if hook_payloads
                .lock()
                .unwrap()
                .iter()
                .any(|payload| payload.get("name").and_then(|n| n.as_str()) == Some("chat"))
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        let payloads = hook_payloads.lock().unwrap().clone();
        assert!(!payloads.is_empty(), "expected webhook payloads");
        let event = payloads
            .iter()
            .find(|v| v.get("name").and_then(|n| n.as_str()) == Some("chat"))
            .cloned()
            .expect("chat webhook not found");
        assert_eq!(
            event.get("topicId").and_then(|v| v.as_str()),
            Some(topic_id)
        );
        assert_eq!(
            event
                .get("data")
                .and_then(|d| d.get("senderId"))
                .and_then(|v| v.as_str()),
            Some("hook-alice")
        );

        server.abort();
        hook_server.abort();
    }

    #[tokio::test]
    async fn api_chat_send_validates_type_and_target() {
        let (app, state) = build_router(test_config()).await.expect("build router");
        let app = app.with_state(state);

        let token = register_and_auth(&app, "validator-user").await;

        let bad_type_req = Request::builder()
            .uri("/api/chat/send")
            .method("POST")
            .header("Authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"type":"typing","message":"nope"}"#))
            .unwrap();
        let bad_type_resp = app.clone().oneshot(bad_type_req).await.unwrap();
        assert_eq!(bad_type_resp.status(), StatusCode::BAD_REQUEST);

        let missing_target_req = Request::builder()
            .uri("/api/chat/send")
            .method("POST")
            .header("Authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"type":"chat","message":"missing target"}"#))
            .unwrap();
        let missing_target_resp = app.oneshot(missing_target_req).await.unwrap();
        assert_eq!(missing_target_resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn openapi_topic_join_requires_user_ids() {
        let (app, state) = build_router(test_config()).await.expect("build router");
        let app = app.with_state(state);

        let create_req = Request::builder()
            .uri("/open/topic/create/topic-join-validate")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"senderId":"join-admin","members":["join-admin"],"multiple":true}"#,
            ))
            .unwrap();
        let create_resp = app.clone().oneshot(create_req).await.unwrap();
        assert_eq!(create_resp.status(), StatusCode::OK);

        let join_req = Request::builder()
            .uri("/open/topic/join/topic-join-validate")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"userIds":[]}"#))
            .unwrap();
        let join_resp = app.oneshot(join_req).await.unwrap();
        assert_eq!(join_resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn openapi_topic_admin_and_transfer_match_go_behavior() {
        let (app, state) = build_router(test_config()).await.expect("build router");
        let app = app.with_state(state);

        let create_req = Request::builder()
            .uri("/open/topic/create/topic-admin-parity")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"senderId":"alice@unittest","members":["alex@unittest","bob@unittest"],"multiple":true}"#,
            ))
            .unwrap();
        let create_resp = app.clone().oneshot(create_req).await.unwrap();
        assert_eq!(create_resp.status(), StatusCode::OK);

        let owner_admin_req = Request::builder()
            .uri("/open/topic/admin/add/topic-admin-parity/alice@unittest")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .body(Body::empty())
            .unwrap();
        let owner_admin_resp = app.clone().oneshot(owner_admin_req).await.unwrap();
        assert_eq!(owner_admin_resp.status(), StatusCode::BAD_REQUEST);

        for user_id in ["alex@unittest", "cc", "alan"] {
            let add_admin_req = Request::builder()
                .uri(format!(
                    "/open/topic/admin/add/topic-admin-parity/{user_id}"
                ))
                .method("POST")
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap();
            let add_admin_resp = app.clone().oneshot(add_admin_req).await.unwrap();
            assert_eq!(add_admin_resp.status(), StatusCode::OK);
        }

        let info_req = Request::builder()
            .uri("/open/topic/info/topic-admin-parity")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .body(Body::empty())
            .unwrap();
        let info_resp = app.clone().oneshot(info_req).await.unwrap();
        assert_eq!(info_resp.status(), StatusCode::OK);
        let info_body = info_resp.into_body().collect().await.unwrap().to_bytes();
        let info_json: serde_json::Value = serde_json::from_slice(&info_body).unwrap();
        assert_eq!(
            info_json.get("admins").and_then(|v| v.as_array()).map(|v| v
                .iter()
                .filter_map(|item| item.as_str())
                .collect::<Vec<_>>()),
            Some(vec!["alex@unittest", "cc", "alan"])
        );

        let remove_admin_req = Request::builder()
            .uri("/open/topic/admin/remove/topic-admin-parity/alan")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .body(Body::empty())
            .unwrap();
        let remove_admin_resp = app.clone().oneshot(remove_admin_req).await.unwrap();
        assert_eq!(remove_admin_resp.status(), StatusCode::OK);

        let transfer_req = Request::builder()
            .uri("/open/topic/transfer/topic-admin-parity/alex@unittest")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .body(Body::empty())
            .unwrap();
        let transfer_resp = app.clone().oneshot(transfer_req).await.unwrap();
        assert_eq!(transfer_resp.status(), StatusCode::OK);

        let info_after_transfer_req = Request::builder()
            .uri("/open/topic/info/topic-admin-parity")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .body(Body::empty())
            .unwrap();
        let info_after_transfer_resp = app.oneshot(info_after_transfer_req).await.unwrap();
        assert_eq!(info_after_transfer_resp.status(), StatusCode::OK);
        let info_after_transfer_body = info_after_transfer_resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let info_after_transfer_json: serde_json::Value =
            serde_json::from_slice(&info_after_transfer_body).unwrap();
        assert_eq!(
            info_after_transfer_json
                .get("ownerId")
                .and_then(|v| v.as_str()),
            Some("alex@unittest")
        );
        assert_eq!(
            info_after_transfer_json
                .get("admins")
                .and_then(|v| v.as_array())
                .map(|v| v
                    .iter()
                    .filter_map(|item| item.as_str())
                    .collect::<Vec<_>>()),
            Some(vec!["cc"])
        );
    }

    #[tokio::test]
    async fn openapi_user_update_relation_and_blacklist_match_go_behavior() {
        let (app, state) = build_router(test_config()).await.expect("build router");
        let db = state.db.clone();
        let app = app.with_state(state);

        let register_req = Request::builder()
            .uri("/open/user/register/hello1")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"displayName":"alice","avatar":"https://bad.png","publicKey":"RSA:1024"}"#,
            ))
            .unwrap();
        let register_resp = app.clone().oneshot(register_req).await.unwrap();
        assert_eq!(register_resp.status(), StatusCode::OK);
        let register_body = register_resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let register_json: serde_json::Value = serde_json::from_slice(&register_body).unwrap();
        assert_eq!(
            register_json.get("name").and_then(|v| v.as_str()),
            Some("alice")
        );
        assert_eq!(
            register_json.get("publicKey").and_then(|v| v.as_str()),
            Some("RSA:1024")
        );

        let update_req = Request::builder()
            .uri("/open/user/update/hello1")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"displayName":"alice2","avatar":"https://bad.png","publicKey":"RSA:2048","password":"xxxxx"}"#,
            ))
            .unwrap();
        let update_resp = app.clone().oneshot(update_req).await.unwrap();
        assert_eq!(update_resp.status(), StatusCode::OK);
        let updated_user = crate::entity::user::Entity::find_by_id("hello1".to_string())
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            updated_user.password,
            crate::api::auth::hash_password("xxxxx")
        );

        let auth_hello1 = register_and_auth(&app, "hello1").await;
        let _auth_alice = register_and_auth(&app, "alice").await;

        let relation_req = Request::builder()
            .uri("/open/user/relation/hello1/alice")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"remark":"ALICE from OpenAPI"}"#))
            .unwrap();
        let relation_resp = app.clone().oneshot(relation_req).await.unwrap();
        assert_eq!(relation_resp.status(), StatusCode::OK);
        let relation_body = relation_resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let relation_json: serde_json::Value = serde_json::from_slice(&relation_body).unwrap();
        assert_eq!(
            relation_json.get("remark").and_then(|v| v.as_str()),
            Some("ALICE from OpenAPI")
        );

        let profile_req = Request::builder()
            .uri("/api/profile/alice")
            .method("POST")
            .header("Authorization", format!("Bearer {auth_hello1}"))
            .body(Body::empty())
            .unwrap();
        let profile_resp = app.clone().oneshot(profile_req).await.unwrap();
        assert_eq!(profile_resp.status(), StatusCode::OK);
        let profile_body = profile_resp.into_body().collect().await.unwrap().to_bytes();
        let profile_json: serde_json::Value = serde_json::from_slice(&profile_body).unwrap();
        assert_eq!(
            profile_json.get("remark").and_then(|v| v.as_str()),
            Some("ALICE from OpenAPI")
        );

        let blacklist_add_req = Request::builder()
            .uri("/open/user/blacklist/add/hello1")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"userIds":["alice","alex"]}"#))
            .unwrap();
        let blacklist_add_resp = app.clone().oneshot(blacklist_add_req).await.unwrap();
        assert_eq!(blacklist_add_resp.status(), StatusCode::OK);
        let blacklist_add_body = blacklist_add_resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let blacklist_add_json: serde_json::Value =
            serde_json::from_slice(&blacklist_add_body).unwrap();
        assert_eq!(blacklist_add_json.as_array().map(|v| v.len()), Some(2));

        let blacklist_get_req = Request::builder()
            .uri("/open/user/blacklist/get/hello1")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .body(Body::empty())
            .unwrap();
        let blacklist_get_resp = app.clone().oneshot(blacklist_get_req).await.unwrap();
        assert_eq!(blacklist_get_resp.status(), StatusCode::OK);
        let blacklist_get_body = blacklist_get_resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let blacklist_get_json: serde_json::Value =
            serde_json::from_slice(&blacklist_get_body).unwrap();
        assert_eq!(blacklist_get_json.as_array().map(|v| v.len()), Some(2));

        let blacklist_remove_req = Request::builder()
            .uri("/open/user/blacklist/remove/hello1")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"userIds":["alice"]}"#))
            .unwrap();
        let blacklist_remove_resp = app.clone().oneshot(blacklist_remove_req).await.unwrap();
        assert_eq!(blacklist_remove_resp.status(), StatusCode::OK);
        let blacklist_remove_body = blacklist_remove_resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let blacklist_remove_json: serde_json::Value =
            serde_json::from_slice(&blacklist_remove_body).unwrap();
        assert_eq!(blacklist_remove_json.as_array().map(|v| v.len()), Some(1));

        let blacklist_get_after_req = Request::builder()
            .uri("/open/user/blacklist/get/hello1")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .body(Body::empty())
            .unwrap();
        let blacklist_get_after_resp = app.oneshot(blacklist_get_after_req).await.unwrap();
        assert_eq!(blacklist_get_after_resp.status(), StatusCode::OK);
        let blacklist_get_after_body = blacklist_get_after_resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let blacklist_get_after_json: serde_json::Value =
            serde_json::from_slice(&blacklist_get_after_body).unwrap();
        assert_eq!(
            blacklist_get_after_json.as_array().map(|v| v.len()),
            Some(1)
        );
    }

    #[tokio::test]
    async fn openapi_topic_create_update_member_and_dismiss_match_go_behavior() {
        let (app, state) = build_router(test_config()).await.expect("build router");
        let app = app.with_state(state);

        for user_id in ["alice@unittest", "alex@unittest", "bob@unittest"] {
            let _ = register_and_auth(&app, user_id).await;
        }

        let create_ensure_req = Request::builder()
            .uri("/open/topic/create/testtopic_ensure")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"senderId":"alice@unittest","members":["alex@unittest","bob@unittest","not_exist_user"],"name":"Test Topic","icon":"https://bad.png","private":true,"knockNeedVerify":true,"ensureConversation":true}"#))
            .unwrap();
        let create_ensure_resp = app.clone().oneshot(create_ensure_req).await.unwrap();
        assert_eq!(create_ensure_resp.status(), StatusCode::OK);

        for user_id in ["bob@unittest", "alex@unittest"] {
            let conv_info_req = Request::builder()
                .uri(format!(
                    "/open/conversation/info/{user_id}/testtopic_ensure"
                ))
                .method("POST")
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap();
            let conv_info_resp = app.clone().oneshot(conv_info_req).await.unwrap();
            assert_eq!(conv_info_resp.status(), StatusCode::OK);
            let conv_info_body = conv_info_resp
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes();
            let conv_info_json: serde_json::Value =
                serde_json::from_slice(&conv_info_body).unwrap();
            assert_eq!(
                conv_info_json.get("name").and_then(|v| v.as_str()),
                Some("Test Topic")
            );
        }

        let create_req = Request::builder()
            .uri("/open/topic/create/testtopic")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"senderId":"alice@unittest","members":["alex@unittest","bob@unittest","not_exist_user"],"name":"Test Topic","icon":"https://bad.png","private":true,"knockNeedVerify":true}"#))
            .unwrap();
        let create_resp = app.clone().oneshot(create_req).await.unwrap();
        assert_eq!(create_resp.status(), StatusCode::OK);
        let create_body = create_resp.into_body().collect().await.unwrap().to_bytes();
        let create_json: serde_json::Value = serde_json::from_slice(&create_body).unwrap();
        assert_eq!(
            create_json.get("id").and_then(|v| v.as_str()),
            Some("testtopic")
        );
        assert_eq!(create_json.get("members").and_then(|v| v.as_u64()), Some(3));
        assert_eq!(
            create_json.get("private").and_then(|v| v.as_bool()),
            Some(true)
        );
        assert_eq!(
            create_json.get("knockNeedVerify").and_then(|v| v.as_bool()),
            Some(true)
        );

        let owner_conv_req = Request::builder()
            .uri("/open/conversation/info/alice@unittest/testtopic")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .body(Body::empty())
            .unwrap();
        let owner_conv_resp = app.clone().oneshot(owner_conv_req).await.unwrap();
        assert_eq!(owner_conv_resp.status(), StatusCode::OK);

        let update_req = Request::builder()
            .uri("/open/topic/update/testtopic")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"name":"Test Topic 2","icon":"https://good.png","admins":["bob@unittest"],"private":false,"knockNeedVerify":false,"webhooks":["http://localhost:8080/webhook"],"notice":{"text":"hello"},"extra":{"bad_json":"{bad_json}"}}"#))
            .unwrap();
        let update_resp = app.clone().oneshot(update_req).await.unwrap();
        assert_eq!(update_resp.status(), StatusCode::OK);

        let topic_info_req = Request::builder()
            .uri("/open/topic/info/testtopic")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .body(Body::empty())
            .unwrap();
        let topic_info_resp = app.clone().oneshot(topic_info_req).await.unwrap();
        assert_eq!(topic_info_resp.status(), StatusCode::OK);
        let topic_info_body = topic_info_resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let topic_info_json: serde_json::Value = serde_json::from_slice(&topic_info_body).unwrap();
        assert_eq!(
            topic_info_json.get("name").and_then(|v| v.as_str()),
            Some("Test Topic 2")
        );
        assert_eq!(
            topic_info_json.get("icon").and_then(|v| v.as_str()),
            Some("https://good.png")
        );
        assert_eq!(
            topic_info_json.get("private").and_then(|v| v.as_bool()),
            Some(false)
        );
        assert_eq!(
            topic_info_json
                .get("knockNeedVerify")
                .and_then(|v| v.as_bool()),
            Some(false)
        );
        assert_eq!(
            topic_info_json
                .get("admins")
                .and_then(|v| v.as_array())
                .map(|v| v
                    .iter()
                    .filter_map(|item| item.as_str())
                    .collect::<Vec<_>>()),
            Some(vec!["bob@unittest"])
        );
        assert_eq!(
            topic_info_json
                .get("notice")
                .and_then(|v| v.get("text"))
                .and_then(|v| v.as_str()),
            Some("hello")
        );
        assert_eq!(
            topic_info_json
                .get("extra")
                .and_then(|v| v.get("bad_json"))
                .and_then(|v| v.as_str()),
            Some("{bad_json}")
        );

        let update_extra_req = Request::builder()
            .uri("/open/topic/update_extra/testtopic")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"actions":[{"action":"remove","key":"bad_json"},{"action":"set","key":"good_json","value":"{}"}]}"#))
            .unwrap();
        let update_extra_resp = app.clone().oneshot(update_extra_req).await.unwrap();
        assert_eq!(update_extra_resp.status(), StatusCode::OK);
        let update_extra_body = update_extra_resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let update_extra_json: serde_json::Value =
            serde_json::from_slice(&update_extra_body).unwrap();
        assert_eq!(
            update_extra_json
                .get("extra")
                .and_then(|v| v.get("good_json"))
                .and_then(|v| v.as_str()),
            Some("{}")
        );
        assert!(update_extra_json
            .get("extra")
            .and_then(|v| v.get("bad_json"))
            .is_none());

        let without_owner_req = Request::builder()
            .uri("/open/topic/create/testtopic_without_owner")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"withoutOwner":true,"members":["alex@unittest","bob@unittest","not_exist_user"],"name":"Test Topic","icon":"https://bad.png","private":true,"knockNeedVerify":true}"#))
            .unwrap();
        let without_owner_resp = app.clone().oneshot(without_owner_req).await.unwrap();
        assert_eq!(without_owner_resp.status(), StatusCode::OK);
        let without_owner_body = without_owner_resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let without_owner_json: serde_json::Value =
            serde_json::from_slice(&without_owner_body).unwrap();
        assert_eq!(
            without_owner_json.get("ownerId").and_then(|v| v.as_str()),
            Some("")
        );
        assert_eq!(
            without_owner_json.get("members").and_then(|v| v.as_u64()),
            Some(2)
        );

        let auto_create_req = Request::builder()
            .uri("/open/topic/create")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"senderId":"alice@unittest","members":["alex@unittest","bob@unittest","not_exist_user"],"name":"Test Topic"}"#))
            .unwrap();
        let auto_create_resp = app.clone().oneshot(auto_create_req).await.unwrap();
        assert_eq!(auto_create_resp.status(), StatusCode::OK);
        let auto_create_body = auto_create_resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let auto_create_json: serde_json::Value =
            serde_json::from_slice(&auto_create_body).unwrap();
        let topic_id = auto_create_json
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap()
            .to_string();

        let join_req = Request::builder()
            .uri(format!("/open/topic/join/{topic_id}"))
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"userIds":["alex@unittest","cc"]}"#))
            .unwrap();
        let join_resp = app.clone().oneshot(join_req).await.unwrap();
        assert_eq!(join_resp.status(), StatusCode::OK);
        let join_body = join_resp.into_body().collect().await.unwrap().to_bytes();
        let join_json: serde_json::Value = serde_json::from_slice(&join_body).unwrap();
        assert_eq!(
            join_json.as_array().map(|v| v
                .iter()
                .filter_map(|item| item.as_str())
                .collect::<Vec<_>>()),
            Some(vec!["alex@unittest", "cc"])
        );

        let member_info_req = Request::builder()
            .uri(format!("/open/topic/member_info/{topic_id}/cc"))
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .body(Body::empty())
            .unwrap();
        let member_info_resp = app.clone().oneshot(member_info_req).await.unwrap();
        assert_eq!(member_info_resp.status(), StatusCode::OK);
        let member_info_body = member_info_resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let member_info_json: serde_json::Value =
            serde_json::from_slice(&member_info_body).unwrap();
        assert_eq!(
            member_info_json.get("userId").and_then(|v| v.as_str()),
            Some("cc")
        );

        let missing_member_info_req = Request::builder()
            .uri(format!("/open/topic/member_info/{topic_id}/cc_not_exist"))
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .body(Body::empty())
            .unwrap();
        let missing_member_info_resp = app.clone().oneshot(missing_member_info_req).await.unwrap();
        assert_eq!(missing_member_info_resp.status(), StatusCode::NOT_FOUND);

        let quit_req = Request::builder()
            .uri(format!("/open/topic/quit/{topic_id}"))
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"userIds":["cc"]}"#))
            .unwrap();
        let quit_resp = app.clone().oneshot(quit_req).await.unwrap();
        assert_eq!(quit_resp.status(), StatusCode::OK);
        let quit_body = quit_resp.into_body().collect().await.unwrap().to_bytes();
        let quit_json: serde_json::Value = serde_json::from_slice(&quit_body).unwrap();
        assert_eq!(quit_json.as_array().map(|v| v.len()), Some(1));

        let members_req = Request::builder()
            .uri(format!("/open/topic/members/{topic_id}"))
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .body(Body::empty())
            .unwrap();
        let members_resp = app.clone().oneshot(members_req).await.unwrap();
        assert_eq!(members_resp.status(), StatusCode::OK);
        let members_body = members_resp.into_body().collect().await.unwrap().to_bytes();
        let members_json: serde_json::Value = serde_json::from_slice(&members_body).unwrap();
        assert_eq!(
            members_json
                .get("items")
                .and_then(|v| v.as_array())
                .map(|v| v.len()),
            Some(3)
        );

        let update_member_req = Request::builder()
            .uri(format!("/open/topic/member/{topic_id}/alex@unittest"))
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"name":"Alex","source":"source_unittest","extra":{"bad_json":"{bad_json}"}}"#,
            ))
            .unwrap();
        let update_member_resp = app.clone().oneshot(update_member_req).await.unwrap();
        assert_eq!(update_member_resp.status(), StatusCode::OK);
        let update_member_body = update_member_resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let update_member_json: serde_json::Value =
            serde_json::from_slice(&update_member_body).unwrap();
        assert_eq!(
            update_member_json.get("name").and_then(|v| v.as_str()),
            Some("Alex")
        );
        assert_eq!(
            update_member_json.get("source").and_then(|v| v.as_str()),
            Some("source_unittest")
        );
        assert_eq!(
            update_member_json
                .get("extra")
                .and_then(|v| v.get("bad_json"))
                .and_then(|v| v.as_str()),
            Some("{bad_json}")
        );

        let dismiss_req = Request::builder()
            .uri(format!("/open/topic/dismiss/{topic_id}"))
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .body(Body::empty())
            .unwrap();
        let dismiss_resp = app.clone().oneshot(dismiss_req).await.unwrap();
        assert_eq!(dismiss_resp.status(), StatusCode::OK);

        let dismissed_info_req = Request::builder()
            .uri(format!("/open/topic/info/{topic_id}"))
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .body(Body::empty())
            .unwrap();
        let dismissed_info_resp = app.oneshot(dismissed_info_req).await.unwrap();
        assert_eq!(dismissed_info_resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn openapi_topic_send_chat_and_conversation_match_go_behavior() {
        let (app, state) = build_router(test_config()).await.expect("build router");
        let app = app.with_state(state);

        for user_id in [
            "alice@unittest",
            "bob@unittest",
            "alice_openapi@unittest",
            "bob_openapi@unittest",
        ] {
            let _ = register_and_auth(&app, user_id).await;
        }

        let create_req = Request::builder()
            .uri("/open/topic/create")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"senderId":"alice@unittest","members":["alex@unittest","bob@unittest","not_exist_user"],"name":"Test Topic"}"#))
            .unwrap();
        let create_resp = app.clone().oneshot(create_req).await.unwrap();
        assert_eq!(create_resp.status(), StatusCode::OK);
        let create_body = create_resp.into_body().collect().await.unwrap().to_bytes();
        let create_json: serde_json::Value = serde_json::from_slice(&create_body).unwrap();
        let topic_id = create_json
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap()
            .to_string();
        assert_eq!(create_json.get("members").and_then(|v| v.as_u64()), Some(2));

        let send_req = Request::builder()
            .uri(format!("/open/topic/send/{topic_id}"))
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"senderId":"alice@unittest","type":"chat","chatId":"mock_chat_id_01","content":{"type":"text","text":"hello"}}"#))
            .unwrap();
        let send_resp = app.clone().oneshot(send_req).await.unwrap();
        assert_eq!(send_resp.status(), StatusCode::OK);
        let send_body = send_resp.into_body().collect().await.unwrap().to_bytes();
        let send_json: serde_json::Value = serde_json::from_slice(&send_body).unwrap();
        assert_eq!(send_json.get("code").and_then(|v| v.as_i64()), Some(200));

        let send_format_req = Request::builder()
            .uri(format!("/open/topic/send/{topic_id}/rongcloud"))
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"senderId":"alice@unittest","message":{"objectName":"RCD:Unittest","content":"{bad json}"}}"#))
            .unwrap();
        let send_format_resp = app.clone().oneshot(send_format_req).await.unwrap();
        assert_eq!(send_format_resp.status(), StatusCode::OK);
        let send_format_body = send_format_resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let send_format_json: serde_json::Value =
            serde_json::from_slice(&send_format_body).unwrap();
        assert_eq!(
            send_format_json.get("code").and_then(|v| v.as_i64()),
            Some(200)
        );

        let create_first_missing_req = Request::builder()
            .uri("/open/topic/send/create_first")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"senderId":"alice@unittest","members":["bob@unittest","not_exist_user"],"name":"TEST","icon":"http://icon","type":"chat","chatId":"mock_chat_id_create","content":{"type":"text","text":"hello"}}"#))
            .unwrap();
        let create_first_missing_resp =
            app.clone().oneshot(create_first_missing_req).await.unwrap();
        assert_eq!(create_first_missing_resp.status(), StatusCode::NOT_FOUND);

        let create_first_ensure_req = Request::builder()
            .uri("/open/topic/send/create_first")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"senderId":"alice@unittest","members":["bob@unittest","not_exist_user"],"name":"TEST","icon":"http://icon","ensure":true,"type":"chat","chatId":"mock_chat_id_create2","content":{"type":"text","text":"hello"}}"#))
            .unwrap();
        let create_first_ensure_resp = app.clone().oneshot(create_first_ensure_req).await.unwrap();
        assert_eq!(create_first_ensure_resp.status(), StatusCode::OK);

        let created_topic_info_req = Request::builder()
            .uri("/open/topic/info/create_first")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .body(Body::empty())
            .unwrap();
        let created_topic_info_resp = app.clone().oneshot(created_topic_info_req).await.unwrap();
        assert_eq!(created_topic_info_resp.status(), StatusCode::OK);
        let created_topic_info_body = created_topic_info_resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let created_topic_info_json: serde_json::Value =
            serde_json::from_slice(&created_topic_info_body).unwrap();
        assert_eq!(
            created_topic_info_json
                .get("ownerId")
                .and_then(|v| v.as_str()),
            Some("alice@unittest")
        );
        assert_eq!(
            created_topic_info_json
                .get("members")
                .and_then(|v| v.as_u64()),
            Some(2)
        );
        assert_eq!(
            created_topic_info_json.get("name").and_then(|v| v.as_str()),
            Some("TEST")
        );
        assert_eq!(
            created_topic_info_json.get("icon").and_then(|v| v.as_str()),
            Some("http://icon")
        );

        let logs_req = Request::builder()
            .uri(format!("/open/topic/logs/{topic_id}"))
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .unwrap();
        let logs_resp = app.clone().oneshot(logs_req).await.unwrap();
        assert_eq!(logs_resp.status(), StatusCode::OK);
        let logs_body = logs_resp.into_body().collect().await.unwrap().to_bytes();
        let logs_json: serde_json::Value = serde_json::from_slice(&logs_body).unwrap();
        assert_eq!(
            logs_json
                .get("items")
                .and_then(|v| v.as_array())
                .map(|v| v.len()),
            Some(2)
        );
        assert_eq!(
            logs_json.get("hasMore").and_then(|v| v.as_bool()),
            Some(false)
        );

        let open_chat_req = Request::builder()
            .uri("/open/chat/bob_openapi@unittest")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"userIds":["alice_openapi@unittest"],"type":"chat","chatId":"mock_chat_id_user_01","content":{"type":"text","text":"hello"}}"#))
            .unwrap();
        let open_chat_resp = app.clone().oneshot(open_chat_req).await.unwrap();
        assert_eq!(open_chat_resp.status(), StatusCode::OK);
        let open_chat_body = open_chat_resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let open_chat_json: serde_json::Value = serde_json::from_slice(&open_chat_body).unwrap();
        assert_eq!(open_chat_json.as_array().map(|v| v.len()), Some(1));
        assert_eq!(
            open_chat_json
                .as_array()
                .and_then(|v| v.first())
                .and_then(|v| v.get("code"))
                .and_then(|v| v.as_i64()),
            Some(200)
        );

        let open_chat_format_req = Request::builder()
            .uri("/open/chat/bob_openapi@unittest/rongcloud")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"userIds":["alice_openapi@unittest"],"message":{"objectName":"RC:TxtMsg","content":"{\"content\":\"hello text\"}"}}"#))
            .unwrap();
        let open_chat_format_resp = app.clone().oneshot(open_chat_format_req).await.unwrap();
        assert_eq!(open_chat_format_resp.status(), StatusCode::OK);
        let open_chat_format_body = open_chat_format_resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let open_chat_format_json: serde_json::Value =
            serde_json::from_slice(&open_chat_format_body).unwrap();
        assert_eq!(open_chat_format_json.as_array().map(|v| v.len()), Some(1));
        assert_eq!(
            open_chat_format_json
                .as_array()
                .and_then(|v| v.first())
                .and_then(|v| v.get("code"))
                .and_then(|v| v.as_i64()),
            Some(200)
        );

        let conv_info_before_req = Request::builder()
            .uri(format!("/open/conversation/info/alice@unittest/{topic_id}"))
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .body(Body::empty())
            .unwrap();
        let conv_info_before_resp = app.clone().oneshot(conv_info_before_req).await.unwrap();
        assert_eq!(conv_info_before_resp.status(), StatusCode::OK);

        let conv_update_req = Request::builder()
            .uri(format!(
                "/open/conversation/update/alice@unittest/{topic_id}"
            ))
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"remark":"openapi remark","mute":true}"#))
            .unwrap();
        let conv_update_resp = app.clone().oneshot(conv_update_req).await.unwrap();
        assert_eq!(conv_update_resp.status(), StatusCode::OK);

        let conv_info_after_req = Request::builder()
            .uri(format!("/open/conversation/info/alice@unittest/{topic_id}"))
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .body(Body::empty())
            .unwrap();
        let conv_info_after_resp = app.clone().oneshot(conv_info_after_req).await.unwrap();
        assert_eq!(conv_info_after_resp.status(), StatusCode::OK);
        let conv_info_after_body = conv_info_after_resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let conv_info_after_json: serde_json::Value =
            serde_json::from_slice(&conv_info_after_body).unwrap();
        assert_eq!(
            conv_info_after_json.get("remark").and_then(|v| v.as_str()),
            Some("openapi remark")
        );
        assert_eq!(
            conv_info_after_json.get("mute").and_then(|v| v.as_bool()),
            Some(true)
        );

        let conv_info_bob_req = Request::builder()
            .uri(format!("/open/conversation/info/bob@unittest/{topic_id}"))
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .body(Body::empty())
            .unwrap();
        let conv_info_bob_resp = app.clone().oneshot(conv_info_bob_req).await.unwrap();
        assert_eq!(conv_info_bob_resp.status(), StatusCode::OK);
        let conv_info_bob_body = conv_info_bob_resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let conv_info_bob_json: serde_json::Value =
            serde_json::from_slice(&conv_info_bob_body).unwrap();
        assert!(conv_info_bob_json
            .get("remark")
            .and_then(|v| v.as_str())
            .is_none_or(|v| v.is_empty()));
        assert_eq!(
            conv_info_bob_json.get("mute").and_then(|v| v.as_bool()),
            Some(false)
        );

        let conv_unread_req = Request::builder()
            .uri(format!(
                "/open/conversation/unread/alice@unittest/{topic_id}"
            ))
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .body(Body::empty())
            .unwrap();
        let conv_unread_resp = app.clone().oneshot(conv_unread_req).await.unwrap();
        assert_eq!(conv_unread_resp.status(), StatusCode::OK);

        let conv_unread_info_req = Request::builder()
            .uri(format!("/open/conversation/info/alice@unittest/{topic_id}"))
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .body(Body::empty())
            .unwrap();
        let conv_unread_info_resp = app.oneshot(conv_unread_info_req).await.unwrap();
        assert_eq!(conv_unread_info_resp.status(), StatusCode::OK);
        let conv_unread_info_body = conv_unread_info_resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let conv_unread_info_json: serde_json::Value =
            serde_json::from_slice(&conv_unread_info_body).unwrap();
        assert_eq!(
            conv_unread_info_json.get("unread").and_then(|v| v.as_i64()),
            Some(1)
        );
    }

    #[tokio::test]
    async fn openapi_user_online_tracks_ws_presence() {
        let config = test_config();
        let (app, state) = build_router(config).await.expect("build router");

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app.with_state(state)).await.unwrap();
        });

        let endpoint = format!("http://{}", addr);
        let client = reqwest::Client::new();

        let _ = client
            .post(format!("{endpoint}/open/user/register/online-user"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();

        let auth_json: serde_json::Value = client
            .post(format!("{endpoint}/open/user/auth/online-user"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let token = auth_json.get("authToken").and_then(|v| v.as_str()).unwrap();

        let mut ws_req = format!("ws://{}/api/connect?device=ios", addr)
            .into_client_request()
            .unwrap();
        ws_req
            .headers_mut()
            .insert("Authorization", format!("Bearer {token}").parse().unwrap());
        let (mut ws, _) = tokio_tungstenite::connect_async(ws_req).await.unwrap();

        let online_resp: serde_json::Value = client
            .post(format!("{endpoint}/open/user/online/online-user"))
            .bearer_auth("test-token")
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(
            online_resp.get("online").and_then(|v| v.as_bool()),
            Some(true)
        );
        assert!(online_resp
            .get("devices")
            .and_then(|v| v.as_array())
            .is_some_and(|arr| arr.iter().any(|v| v.as_str() == Some("ios"))));

        let _ = ws.close(None).await;
        drop(ws);

        let mut final_online = true;
        let mut final_devices = Vec::new();
        for _ in 0..30 {
            let resp: serde_json::Value = client
                .post(format!("{endpoint}/open/user/online/online-user"))
                .bearer_auth("test-token")
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
            final_online = resp
                .get("online")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            final_devices = resp
                .get("devices")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            if !final_online {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        assert!(!final_online, "user should be offline after ws close");
        assert!(
            final_devices.is_empty(),
            "devices should be empty after ws close"
        );

        server.abort();
    }

    #[tokio::test]
    async fn openapi_conversation_update_pushes_system_chat_to_owner() {
        let config = test_config();
        let (app, state) = build_router(config).await.expect("build router");

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app.with_state(state)).await.unwrap();
        });

        let endpoint = format!("http://{}", addr);
        let client = reqwest::Client::new();

        let owner_token = register_and_auth_http(&client, &endpoint, "conv-push-a").await;
        let _other_token = register_and_auth_http(&client, &endpoint, "conv-push-b").await;

        let topic_resp: serde_json::Value = client
            .post(format!("{endpoint}/open/topic/create/conv-push-topic"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({
                "senderId": "conv-push-a",
                "members": ["conv-push-b"],
                "multiple": true,
                "ensureConversation": true,
            }))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let topic_id = topic_resp
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap()
            .to_string();

        let mut ws_req = format!("ws://{}/api/connect?device=conv-owner", addr)
            .into_client_request()
            .unwrap();
        ws_req.headers_mut().insert(
            "Authorization",
            format!("Bearer {owner_token}").parse().unwrap(),
        );
        let (mut owner_ws, _) = tokio_tungstenite::connect_async(ws_req).await.unwrap();

        let update_resp = client
            .post(format!(
                "{endpoint}/open/conversation/update/conv-push-a/{topic_id}"
            ))
            .bearer_auth("test-token")
            .json(&serde_json::json!({"sticky": true, "remark": "pin me"}))
            .send()
            .await
            .unwrap();
        assert_eq!(update_resp.status(), reqwest::StatusCode::OK);

        let pushed = tokio::time::timeout(std::time::Duration::from_secs(3), owner_ws.next())
            .await
            .expect("conversation update push timeout")
            .expect("conversation update stream ended")
            .expect("conversation update ws error")
            .into_text()
            .unwrap();
        let pushed_json: serde_json::Value = serde_json::from_str(&pushed).unwrap();
        assert_eq!(
            pushed_json.get("type").and_then(|v| v.as_str()),
            Some("chat")
        );
        assert_eq!(
            pushed_json.get("topicId").and_then(|v| v.as_str()),
            Some(topic_id.as_str())
        );
        assert_eq!(
            pushed_json
                .get("content")
                .and_then(|v| v.get("type"))
                .and_then(|v| v.as_str()),
            Some("conversation.update")
        );
        let fields: serde_json::Value = serde_json::from_str(
            pushed_json
                .get("content")
                .and_then(|v| v.get("text"))
                .and_then(|v| v.as_str())
                .unwrap(),
        )
        .unwrap();
        assert_eq!(fields.get("sticky").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(
            fields.get("remark").and_then(|v| v.as_str()),
            Some("pin me")
        );
        assert_eq!(
            pushed_json
                .get("content")
                .and_then(|v| v.get("unreadable"))
                .and_then(|v| v.as_bool()),
            Some(true)
        );

        let _ = owner_ws.close(None).await;
        server.abort();
    }

    #[tokio::test]
    async fn typing_event_does_not_trigger_webhook() {
        use std::sync::{Arc, Mutex};

        let hook_payloads: Arc<Mutex<Vec<serde_json::Value>>> = Arc::new(Mutex::new(Vec::new()));
        let hook_payloads_cloned = hook_payloads.clone();

        let hook_app = axum::Router::new().route(
            "/hook",
            axum::routing::post(move |body: String| {
                let hook_payloads = hook_payloads_cloned.clone();
                async move {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&body) {
                        hook_payloads.lock().unwrap().push(value);
                    }
                    axum::http::StatusCode::OK
                }
            }),
        );

        let hook_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let hook_addr = hook_listener.local_addr().unwrap();
        let hook_server = tokio::spawn(async move {
            axum::serve(hook_listener, hook_app).await.unwrap();
        });

        let config = test_config();
        let (app, state) = build_router(config).await.expect("build router");

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app.with_state(state)).await.unwrap();
        });

        let endpoint = format!("http://{}", addr);
        let client = reqwest::Client::new();

        let _ = client
            .post(format!("{endpoint}/open/user/register/type-a"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();
        let _ = client
            .post(format!("{endpoint}/open/user/register/type-b"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();

        let auth_json: serde_json::Value = client
            .post(format!("{endpoint}/open/user/auth/type-a"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let token = auth_json.get("authToken").and_then(|v| v.as_str()).unwrap();

        let webhook_url = format!("http://{}/hook", hook_addr);
        let topic_resp: serde_json::Value = client
            .post(format!("{endpoint}/open/topic/create/topic-typing-hook"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({
                "senderId": "type-a",
                "members": ["type-a", "type-b"],
                "multiple": true,
                "webhooks": [webhook_url]
            }))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let topic_id = topic_resp.get("id").and_then(|v| v.as_str()).unwrap();

        hook_payloads.lock().unwrap().clear();

        let mut ws_req = format!("ws://{}/api/connect?device=typing-a", addr)
            .into_client_request()
            .unwrap();
        ws_req
            .headers_mut()
            .insert("Authorization", format!("Bearer {token}").parse().unwrap());
        let (mut ws, _) = tokio_tungstenite::connect_async(ws_req).await.unwrap();

        ws.send(tokio_tungstenite::tungstenite::Message::Text(
            serde_json::json!({
                "type": "typing",
                "topicId": topic_id,
                "attendee": "type-b"
            })
            .to_string(),
        ))
        .await
        .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        let payloads = hook_payloads.lock().unwrap().clone();
        assert!(
            !payloads
                .iter()
                .any(|v| v.get("name").and_then(|n| n.as_str()) == Some("typing")),
            "typing should not trigger webhook delivery"
        );

        let _ = ws.close(None).await;
        server.abort();
        hook_server.abort();
    }

    #[tokio::test]
    async fn webhook_read_event_payload_is_delivered() {
        use std::sync::{Arc, Mutex};

        let hook_payloads: Arc<Mutex<Vec<serde_json::Value>>> = Arc::new(Mutex::new(Vec::new()));
        let hook_payloads_cloned = hook_payloads.clone();

        let hook_app = axum::Router::new().route(
            "/hook",
            axum::routing::post(move |body: String| {
                let hook_payloads = hook_payloads_cloned.clone();
                async move {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&body) {
                        hook_payloads.lock().unwrap().push(value);
                    }
                    axum::http::StatusCode::OK
                }
            }),
        );

        let hook_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let hook_addr = hook_listener.local_addr().unwrap();
        let hook_server = tokio::spawn(async move {
            axum::serve(hook_listener, hook_app).await.unwrap();
        });

        let config = test_config();
        let (app, state) = build_router(config).await.expect("build router");

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app.with_state(state)).await.unwrap();
        });

        let endpoint = format!("http://{}", addr);
        let client = reqwest::Client::new();

        let _ = client
            .post(format!("{endpoint}/open/user/register/read-a"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();
        let _ = client
            .post(format!("{endpoint}/open/user/register/read-b"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();

        let auth_json: serde_json::Value = client
            .post(format!("{endpoint}/open/user/auth/read-a"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let token = auth_json.get("authToken").and_then(|v| v.as_str()).unwrap();

        let webhook_url = format!("http://{}/hook", hook_addr);
        let topic_resp: serde_json::Value = client
            .post(format!("{endpoint}/open/topic/create/topic-read-hook"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({
                "senderId": "read-a",
                "members": ["read-a", "read-b"],
                "multiple": true,
                "webhooks": [webhook_url]
            }))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let topic_id = topic_resp.get("id").and_then(|v| v.as_str()).unwrap();

        let send_resp = client
            .post(format!("{endpoint}/open/topic/send/{topic_id}"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({
                "senderId": "read-a",
                "type": "chat",
                "chatId": "read-chat-1",
                "message": "seed read"
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(send_resp.status(), reqwest::StatusCode::OK);

        hook_payloads.lock().unwrap().clear();

        let read_resp = client
            .post(format!("{endpoint}/api/chat/read/{topic_id}"))
            .bearer_auth(token)
            .send()
            .await
            .unwrap();
        assert_eq!(read_resp.status(), reqwest::StatusCode::OK);

        for _ in 0..50 {
            if hook_payloads
                .lock()
                .unwrap()
                .iter()
                .any(|payload| payload.get("name").and_then(|n| n.as_str()) == Some("read"))
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        let payloads = hook_payloads.lock().unwrap().clone();
        let event = payloads
            .iter()
            .find(|v| v.get("name").and_then(|n| n.as_str()) == Some("read"))
            .cloned()
            .expect("read webhook not found");
        assert_eq!(
            event.get("topicId").and_then(|v| v.as_str()),
            Some(topic_id)
        );
        assert_eq!(
            event
                .get("data")
                .and_then(|d| d.get("topicId"))
                .and_then(|v| v.as_str()),
            Some(topic_id)
        );
        assert_eq!(
            event
                .get("data")
                .and_then(|d| d.get("userId"))
                .and_then(|v| v.as_str()),
            Some("read-a")
        );
        assert!(event
            .get("data")
            .and_then(|d| d.get("lastReadSeq"))
            .and_then(|v| v.as_i64())
            .is_some_and(|seq| seq > 0));

        server.abort();
        hook_server.abort();
    }

    #[tokio::test]
    async fn webhook_conversation_update_uses_global_targets_only() {
        use std::sync::{Arc, Mutex};

        let global_payloads: Arc<Mutex<Vec<serde_json::Value>>> = Arc::new(Mutex::new(Vec::new()));
        let global_payloads_cloned = global_payloads.clone();
        let global_app = axum::Router::new().route(
            "/hook",
            axum::routing::post(move |body: String| {
                let payloads = global_payloads_cloned.clone();
                async move {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&body) {
                        payloads.lock().unwrap().push(value);
                    }
                    axum::http::StatusCode::OK
                }
            }),
        );

        let topic_payloads: Arc<Mutex<Vec<serde_json::Value>>> = Arc::new(Mutex::new(Vec::new()));
        let topic_payloads_cloned = topic_payloads.clone();
        let topic_app = axum::Router::new().route(
            "/hook",
            axum::routing::post(move |body: String| {
                let payloads = topic_payloads_cloned.clone();
                async move {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&body) {
                        payloads.lock().unwrap().push(value);
                    }
                    axum::http::StatusCode::OK
                }
            }),
        );

        let global_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let global_addr = global_listener.local_addr().unwrap();
        let global_server = tokio::spawn(async move {
            axum::serve(global_listener, global_app).await.unwrap();
        });

        let topic_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let topic_addr = topic_listener.local_addr().unwrap();
        let topic_server = tokio::spawn(async move {
            axum::serve(topic_listener, topic_app).await.unwrap();
        });

        let mut config = test_config();
        config.webhook_targets = vec![format!("http://{}/hook", global_addr)];
        let (app, state) = build_router(config).await.expect("build router");

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app.with_state(state)).await.unwrap();
        });

        let endpoint = format!("http://{}", addr);
        let client = reqwest::Client::new();

        let _ = client
            .post(format!("{endpoint}/open/user/register/conv-a"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();
        let _ = client
            .post(format!("{endpoint}/open/user/register/conv-b"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();

        let topic_webhook_url = format!("http://{}/hook", topic_addr);
        let topic_resp: serde_json::Value = client
            .post(format!("{endpoint}/open/topic/create/topic-conv-hook"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({
                "senderId": "conv-a",
                "members": ["conv-a", "conv-b"],
                "multiple": true,
                "webhooks": [topic_webhook_url]
            }))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let topic_id = topic_resp
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap()
            .to_string();

        let send_resp = client
            .post(format!("{endpoint}/open/topic/send/{topic_id}"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({
                "senderId": "conv-a",
                "type": "chat",
                "chatId": "conv-chat-1",
                "message": "seed conversation"
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(send_resp.status(), reqwest::StatusCode::OK);

        global_payloads.lock().unwrap().clear();
        topic_payloads.lock().unwrap().clear();

        let update_resp = client
            .post(format!(
                "{endpoint}/open/conversation/update/conv-a/{topic_id}"
            ))
            .bearer_auth("test-token")
            .json(&serde_json::json!({"sticky": true, "remark": "pin"}))
            .send()
            .await
            .unwrap();
        assert_eq!(update_resp.status(), reqwest::StatusCode::OK);

        for _ in 0..50 {
            if global_payloads.lock().unwrap().iter().any(|payload| {
                payload.get("name").and_then(|n| n.as_str()) == Some("conversation.update")
            }) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        let global_events = global_payloads.lock().unwrap().clone();
        let update_event = global_events
            .iter()
            .find(|v| v.get("name").and_then(|n| n.as_str()) == Some("conversation.update"))
            .cloned()
            .expect("conversation.update webhook not found in global targets");

        assert_eq!(
            update_event
                .get("data")
                .and_then(|d| d.get("ownerId"))
                .and_then(|v| v.as_str()),
            Some("conv-a")
        );
        assert_eq!(
            update_event
                .get("data")
                .and_then(|d| d.get("topicId"))
                .and_then(|v| v.as_str()),
            Some(topic_id.as_str())
        );

        let topic_events = topic_payloads.lock().unwrap().clone();
        assert!(
            !topic_events
                .iter()
                .any(|v| { v.get("name").and_then(|n| n.as_str()) == Some("conversation.update") }),
            "conversation.update should not be delivered to topic webhooks"
        );

        server.abort();
        global_server.abort();
        topic_server.abort();
    }

    #[tokio::test]
    async fn webhook_topic_admin_events_payload_shape_parity() {
        use std::sync::{Arc, Mutex};

        let hook_payloads: Arc<Mutex<Vec<serde_json::Value>>> = Arc::new(Mutex::new(Vec::new()));
        let hook_payloads_cloned = hook_payloads.clone();

        let hook_app = axum::Router::new().route(
            "/hook",
            axum::routing::post(move |body: String| {
                let payloads = hook_payloads_cloned.clone();
                async move {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&body) {
                        payloads.lock().unwrap().push(value);
                    }
                    axum::http::StatusCode::OK
                }
            }),
        );

        let hook_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let hook_addr = hook_listener.local_addr().unwrap();
        let hook_server = tokio::spawn(async move {
            axum::serve(hook_listener, hook_app).await.unwrap();
        });

        let config = test_config();
        let (app, state) = build_router(config).await.expect("build router");

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app.with_state(state)).await.unwrap();
        });

        let endpoint = format!("http://{}", addr);
        let client = reqwest::Client::new();

        let owner_token = register_and_auth_http(&client, &endpoint, "hook-owner").await;
        let _target_token = register_and_auth_http(&client, &endpoint, "hook-target").await;
        let applicant_token = register_and_auth_http(&client, &endpoint, "hook-applicant").await;

        let webhook_url = format!("http://{}/hook", hook_addr);
        let create_resp: serde_json::Value = client
            .post(format!("{endpoint}/api/topic/create"))
            .bearer_auth(&owner_token)
            .json(&serde_json::json!({
                "name": "topic-webhook-admin-events",
                "members": ["hook-owner", "hook-target"],
                "multiple": true,
                "knockNeedVerify": true,
                "webhooks": [webhook_url]
            }))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let topic_id = create_resp
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap()
            .to_string();

        let knock_resp = client
            .post(format!("{endpoint}/api/topic/knock/{topic_id}"))
            .bearer_auth(&applicant_token)
            .json(&serde_json::json!({"message": "please approve", "source": "mobile"}))
            .send()
            .await
            .unwrap();
        assert_eq!(knock_resp.status(), reqwest::StatusCode::OK);

        hook_payloads.lock().unwrap().clear();

        let accept_resp = client
            .post(format!(
                "{endpoint}/api/topic/admin/knock/accept/{topic_id}/hook-applicant"
            ))
            .bearer_auth(&owner_token)
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();
        assert_eq!(accept_resp.status(), reqwest::StatusCode::OK);

        for _ in 0..50 {
            if hook_payloads.lock().unwrap().iter().any(|payload| {
                payload.get("name").and_then(|n| n.as_str()) == Some("topic.knock.accept")
            }) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        let payloads_after_accept = hook_payloads.lock().unwrap().clone();
        let knock_accept_event = payloads_after_accept
            .iter()
            .find(|v| v.get("name").and_then(|n| n.as_str()) == Some("topic.knock.accept"))
            .cloned()
            .expect("topic.knock.accept webhook not found");
        assert_eq!(
            knock_accept_event.get("topicId").and_then(|v| v.as_str()),
            Some(topic_id.as_str())
        );
        assert_eq!(
            knock_accept_event
                .get("data")
                .and_then(|d| d.get("topicId"))
                .and_then(|v| v.as_str()),
            Some(topic_id.as_str())
        );
        assert_eq!(
            knock_accept_event
                .get("data")
                .and_then(|d| d.get("adminId"))
                .and_then(|v| v.as_str()),
            Some("hook-owner")
        );
        assert_eq!(
            knock_accept_event
                .get("data")
                .and_then(|d| d.get("userId"))
                .and_then(|v| v.as_str()),
            Some("hook-applicant")
        );
        assert_eq!(
            knock_accept_event
                .get("data")
                .and_then(|d| d.get("source"))
                .and_then(|v| v.as_str()),
            Some("api")
        );

        hook_payloads.lock().unwrap().clear();

        let silent_resp = client
            .post(format!(
                "{endpoint}/api/topic/admin/silent/{topic_id}/hook-target"
            ))
            .bearer_auth(&owner_token)
            .json(&serde_json::json!({"duration": "5m"}))
            .send()
            .await
            .unwrap();
        assert_eq!(silent_resp.status(), reqwest::StatusCode::OK);

        for _ in 0..50 {
            if hook_payloads.lock().unwrap().iter().any(|payload| {
                payload.get("name").and_then(|n| n.as_str()) == Some("topic.silent.member")
            }) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        let payloads_after_silent = hook_payloads.lock().unwrap().clone();
        let silent_event = payloads_after_silent
            .iter()
            .find(|v| v.get("name").and_then(|n| n.as_str()) == Some("topic.silent.member"))
            .cloned()
            .expect("topic.silent.member webhook not found");
        assert_eq!(
            silent_event
                .get("data")
                .and_then(|d| d.get("adminId"))
                .and_then(|v| v.as_str()),
            Some("hook-owner")
        );
        assert_eq!(
            silent_event
                .get("data")
                .and_then(|d| d.get("userId"))
                .and_then(|v| v.as_str()),
            Some("hook-target")
        );
        assert_eq!(
            silent_event
                .get("data")
                .and_then(|d| d.get("duration"))
                .and_then(|v| v.as_str()),
            Some("5m")
        );
        assert_eq!(
            silent_event
                .get("data")
                .and_then(|d| d.get("source"))
                .and_then(|v| v.as_str()),
            Some("api")
        );

        hook_payloads.lock().unwrap().clear();

        let transfer_resp = client
            .post(format!(
                "{endpoint}/api/topic/admin/transfer/{topic_id}/hook-target"
            ))
            .bearer_auth(&owner_token)
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();
        assert_eq!(transfer_resp.status(), reqwest::StatusCode::OK);

        for _ in 0..50 {
            if hook_payloads.lock().unwrap().iter().any(|payload| {
                payload.get("name").and_then(|n| n.as_str()) == Some("topic.changeowner")
            }) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        let payloads_after_transfer = hook_payloads.lock().unwrap().clone();
        let change_owner_event = payloads_after_transfer
            .iter()
            .find(|v| v.get("name").and_then(|n| n.as_str()) == Some("topic.changeowner"))
            .cloned()
            .expect("topic.changeowner webhook not found");
        assert_eq!(
            change_owner_event
                .get("data")
                .and_then(|d| d.get("adminId"))
                .and_then(|v| v.as_str()),
            Some("hook-owner")
        );
        assert_eq!(
            change_owner_event
                .get("data")
                .and_then(|d| d.get("userId"))
                .and_then(|v| v.as_str()),
            Some("hook-target")
        );
        assert_eq!(
            change_owner_event
                .get("data")
                .and_then(|d| d.get("source"))
                .and_then(|v| v.as_str()),
            Some("api")
        );

        server.abort();
        hook_server.abort();
    }

    #[tokio::test]
    async fn webhook_topic_reject_notice_kickout_payload_shape_parity() {
        use std::sync::{Arc, Mutex};

        let hook_payloads: Arc<Mutex<Vec<serde_json::Value>>> = Arc::new(Mutex::new(Vec::new()));
        let hook_payloads_cloned = hook_payloads.clone();

        let hook_app = axum::Router::new().route(
            "/hook",
            axum::routing::post(move |body: String| {
                let payloads = hook_payloads_cloned.clone();
                async move {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&body) {
                        payloads.lock().unwrap().push(value);
                    }
                    axum::http::StatusCode::OK
                }
            }),
        );

        let hook_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let hook_addr = hook_listener.local_addr().unwrap();
        let hook_server = tokio::spawn(async move {
            axum::serve(hook_listener, hook_app).await.unwrap();
        });

        let config = test_config();
        let (app, state) = build_router(config).await.expect("build router");

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app.with_state(state)).await.unwrap();
        });

        let endpoint = format!("http://{}", addr);
        let client = reqwest::Client::new();

        let owner_token = register_and_auth_http(&client, &endpoint, "hook2-owner").await;
        let _member_token = register_and_auth_http(&client, &endpoint, "hook2-member").await;
        let applicant_token = register_and_auth_http(&client, &endpoint, "hook2-applicant").await;

        let webhook_url = format!("http://{}/hook", hook_addr);
        let create_resp: serde_json::Value = client
            .post(format!("{endpoint}/api/topic/create"))
            .bearer_auth(&owner_token)
            .json(&serde_json::json!({
                "name": "topic-webhook-admin-events-2",
                "members": ["hook2-owner", "hook2-member"],
                "multiple": true,
                "knockNeedVerify": true,
                "webhooks": [webhook_url]
            }))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let topic_id = create_resp
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap()
            .to_string();

        let knock_resp = client
            .post(format!("{endpoint}/api/topic/knock/{topic_id}"))
            .bearer_auth(&applicant_token)
            .json(&serde_json::json!({"message": "please reject", "source": "mobile"}))
            .send()
            .await
            .unwrap();
        assert_eq!(knock_resp.status(), reqwest::StatusCode::OK);

        hook_payloads.lock().unwrap().clear();

        let reject_resp = client
            .post(format!(
                "{endpoint}/api/topic/admin/knock/reject/{topic_id}/hook2-applicant"
            ))
            .bearer_auth(&owner_token)
            .json(&serde_json::json!({"message": "declined"}))
            .send()
            .await
            .unwrap();
        assert_eq!(reject_resp.status(), reqwest::StatusCode::OK);

        for _ in 0..50 {
            if hook_payloads.lock().unwrap().iter().any(|payload| {
                payload.get("name").and_then(|n| n.as_str()) == Some("topic.knock.reject")
            }) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        let payloads_after_reject = hook_payloads.lock().unwrap().clone();
        let reject_event = payloads_after_reject
            .iter()
            .find(|v| v.get("name").and_then(|n| n.as_str()) == Some("topic.knock.reject"))
            .cloned()
            .expect("topic.knock.reject webhook not found");
        assert_eq!(
            reject_event.get("topicId").and_then(|v| v.as_str()),
            Some(topic_id.as_str())
        );
        assert_eq!(
            reject_event
                .get("data")
                .and_then(|d| d.get("adminId"))
                .and_then(|v| v.as_str()),
            Some("hook2-owner")
        );
        assert_eq!(
            reject_event
                .get("data")
                .and_then(|d| d.get("userId"))
                .and_then(|v| v.as_str()),
            Some("hook2-applicant")
        );
        assert_eq!(
            reject_event
                .get("data")
                .and_then(|d| d.get("message"))
                .and_then(|v| v.as_str()),
            Some("declined")
        );
        assert_eq!(
            reject_event
                .get("data")
                .and_then(|d| d.get("source"))
                .and_then(|v| v.as_str()),
            Some("api")
        );

        hook_payloads.lock().unwrap().clear();

        let notice_resp = client
            .post(format!("{endpoint}/api/topic/admin/notice/{topic_id}"))
            .bearer_auth(&owner_token)
            .json(&serde_json::json!({"text": "new notice"}))
            .send()
            .await
            .unwrap();
        assert_eq!(notice_resp.status(), reqwest::StatusCode::OK);

        for _ in 0..50 {
            if hook_payloads
                .lock()
                .unwrap()
                .iter()
                .any(|payload| payload.get("name").and_then(|n| n.as_str()) == Some("topic.notice"))
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        let payloads_after_notice = hook_payloads.lock().unwrap().clone();
        let notice_event = payloads_after_notice
            .iter()
            .find(|v| v.get("name").and_then(|n| n.as_str()) == Some("topic.notice"))
            .cloned()
            .expect("topic.notice webhook not found");
        assert_eq!(
            notice_event
                .get("data")
                .and_then(|d| d.get("adminId"))
                .and_then(|v| v.as_str()),
            Some("hook2-owner")
        );
        assert_eq!(
            notice_event
                .get("data")
                .and_then(|d| d.get("message"))
                .and_then(|v| v.as_str()),
            Some("notice.update")
        );

        hook_payloads.lock().unwrap().clear();

        let kickout_resp = client
            .post(format!(
                "{endpoint}/api/topic/admin/kickout/{topic_id}/hook2-member"
            ))
            .bearer_auth(&owner_token)
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();
        assert_eq!(kickout_resp.status(), reqwest::StatusCode::OK);

        for _ in 0..50 {
            if hook_payloads.lock().unwrap().iter().any(|payload| {
                payload.get("name").and_then(|n| n.as_str()) == Some("topic.kickout")
            }) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        let payloads_after_kickout = hook_payloads.lock().unwrap().clone();
        let kickout_event = payloads_after_kickout
            .iter()
            .find(|v| v.get("name").and_then(|n| n.as_str()) == Some("topic.kickout"))
            .cloned()
            .expect("topic.kickout webhook not found");
        assert_eq!(
            kickout_event
                .get("data")
                .and_then(|d| d.get("adminId"))
                .and_then(|v| v.as_str()),
            Some("hook2-owner")
        );
        assert_eq!(
            kickout_event
                .get("data")
                .and_then(|d| d.get("userId"))
                .and_then(|v| v.as_str()),
            Some("hook2-member")
        );
        assert_eq!(
            kickout_event
                .get("data")
                .and_then(|d| d.get("source"))
                .and_then(|v| v.as_str()),
            Some("api")
        );

        server.abort();
        hook_server.abort();
    }

    #[tokio::test]
    async fn webhook_topic_core_event_payload_shape_parity() {
        use std::sync::{Arc, Mutex};

        let hook_payloads: Arc<Mutex<Vec<serde_json::Value>>> = Arc::new(Mutex::new(Vec::new()));
        let hook_payloads_cloned = hook_payloads.clone();

        let hook_app = axum::Router::new().route(
            "/hook",
            axum::routing::post(move |body: String| {
                let payloads = hook_payloads_cloned.clone();
                async move {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&body) {
                        payloads.lock().unwrap().push(value);
                    }
                    axum::http::StatusCode::OK
                }
            }),
        );

        let hook_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let hook_addr = hook_listener.local_addr().unwrap();
        let hook_server = tokio::spawn(async move {
            axum::serve(hook_listener, hook_app).await.unwrap();
        });

        let config = test_config();
        let (app, state) = build_router(config).await.expect("build router");

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app.with_state(state)).await.unwrap();
        });

        let endpoint = format!("http://{}", addr);
        let client = reqwest::Client::new();

        let owner_token = register_and_auth_http(&client, &endpoint, "hook3-owner").await;
        let member_token = register_and_auth_http(&client, &endpoint, "hook3-member").await;
        let joiner_token = register_and_auth_http(&client, &endpoint, "hook3-joiner").await;

        let webhook_url = format!("http://{}/hook", hook_addr);
        let create_resp: serde_json::Value = client
            .post(format!("{endpoint}/api/topic/create"))
            .bearer_auth(&owner_token)
            .json(&serde_json::json!({
                "name": "topic-webhook-core-events",
                "members": ["hook3-owner", "hook3-member"],
                "multiple": true,
                "knockNeedVerify": true,
                "webhooks": [webhook_url]
            }))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let topic_id = create_resp
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap()
            .to_string();

        for _ in 0..50 {
            if hook_payloads
                .lock()
                .unwrap()
                .iter()
                .any(|payload| payload.get("name").and_then(|n| n.as_str()) == Some("topic.create"))
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        let payloads_after_create = hook_payloads.lock().unwrap().clone();
        let create_event = payloads_after_create
            .iter()
            .find(|v| v.get("name").and_then(|n| n.as_str()) == Some("topic.create"))
            .cloned()
            .expect("topic.create webhook not found");
        assert_eq!(
            create_event
                .get("data")
                .and_then(|d| d.get("topicId"))
                .and_then(|v| v.as_str()),
            Some(topic_id.as_str())
        );
        assert_eq!(
            create_event
                .get("data")
                .and_then(|d| d.get("adminId"))
                .and_then(|v| v.as_str()),
            Some("hook3-owner")
        );

        hook_payloads.lock().unwrap().clear();

        let update_resp = client
            .post(format!("{endpoint}/api/topic/admin/update/{topic_id}"))
            .bearer_auth(&owner_token)
            .json(&serde_json::json!({"name": "topic-webhook-core-events-updated"}))
            .send()
            .await
            .unwrap();
        assert_eq!(update_resp.status(), reqwest::StatusCode::OK);

        let join_resp = client
            .post(format!("{endpoint}/api/topic/knock/{topic_id}"))
            .bearer_auth(&joiner_token)
            .json(&serde_json::json!({"message": "hello hook", "source": "mobile"}))
            .send()
            .await
            .unwrap();
        assert_eq!(join_resp.status(), reqwest::StatusCode::OK);

        let accept_resp = client
            .post(format!(
                "{endpoint}/api/topic/admin/knock/accept/{topic_id}/hook3-joiner"
            ))
            .bearer_auth(&owner_token)
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();
        assert_eq!(accept_resp.status(), reqwest::StatusCode::OK);

        let invite_resp = client
            .post(format!(
                "{endpoint}/api/topic/invite/{topic_id}/hook3-member"
            ))
            .bearer_auth(&owner_token)
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();
        assert_eq!(invite_resp.status(), reqwest::StatusCode::OK);

        let silent_topic_resp = client
            .post(format!(
                "{endpoint}/api/topic/admin/silent_topic/{topic_id}"
            ))
            .bearer_auth(&owner_token)
            .json(&serde_json::json!({"duration": "forever"}))
            .send()
            .await
            .unwrap();
        assert_eq!(silent_topic_resp.status(), reqwest::StatusCode::OK);

        let quit_resp = client
            .post(format!("{endpoint}/api/topic/quit/{topic_id}"))
            .bearer_auth(&member_token)
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();
        assert_eq!(quit_resp.status(), reqwest::StatusCode::OK);

        let dismiss_resp = client
            .post(format!("{endpoint}/api/topic/dismiss/{topic_id}"))
            .bearer_auth(&owner_token)
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();
        assert_eq!(dismiss_resp.status(), reqwest::StatusCode::OK);

        for _ in 0..50 {
            let payloads = hook_payloads.lock().unwrap().clone();
            let names = payloads
                .iter()
                .filter_map(|payload| payload.get("name").and_then(|n| n.as_str()))
                .collect::<Vec<_>>();
            if names.contains(&"topic.update")
                && names.contains(&"topic.knock")
                && names.contains(&"topic.join")
                && names.contains(&"topic.silent")
                && names.contains(&"topic.quit")
                && names.contains(&"topic.dismiss")
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        let payloads = hook_payloads.lock().unwrap().clone();
        let find_event = |name: &str| {
            payloads
                .iter()
                .find(|v| v.get("name").and_then(|n| n.as_str()) == Some(name))
                .cloned()
                .unwrap_or_else(|| panic!("{name} webhook not found"))
        };

        let update_event = find_event("topic.update");
        assert_eq!(
            update_event
                .get("data")
                .and_then(|d| d.get("adminId"))
                .and_then(|v| v.as_str()),
            Some("hook3-owner")
        );

        let knock_event = find_event("topic.knock");
        assert_eq!(
            knock_event
                .get("data")
                .and_then(|d| d.get("userId"))
                .and_then(|v| v.as_str()),
            Some("hook3-joiner")
        );
        assert_eq!(
            knock_event
                .get("data")
                .and_then(|d| d.get("message"))
                .and_then(|v| v.as_str()),
            Some("hello hook")
        );

        let join_event = find_event("topic.join");
        assert_eq!(
            join_event
                .get("data")
                .and_then(|d| d.get("userId"))
                .and_then(|v| v.as_str()),
            Some("hook3-member")
        );

        let silent_event = find_event("topic.silent");
        assert_eq!(
            silent_event
                .get("data")
                .and_then(|d| d.get("adminId"))
                .and_then(|v| v.as_str()),
            Some("hook3-owner")
        );
        assert_eq!(
            silent_event
                .get("data")
                .and_then(|d| d.get("duration"))
                .and_then(|v| v.as_str()),
            Some("forever")
        );

        let quit_event = find_event("topic.quit");
        assert_eq!(
            quit_event
                .get("data")
                .and_then(|d| d.get("userId"))
                .and_then(|v| v.as_str()),
            Some("hook3-member")
        );

        let dismiss_event = find_event("topic.dismiss");
        assert_eq!(
            dismiss_event
                .get("data")
                .and_then(|d| d.get("topicId"))
                .and_then(|v| v.as_str()),
            Some(topic_id.as_str())
        );

        server.abort();
        hook_server.abort();
    }

    #[tokio::test]
    async fn webhook_upload_file_and_guest_create_payload_shape_parity() {
        use std::sync::{Arc, Mutex};

        let global_payloads: Arc<Mutex<Vec<serde_json::Value>>> = Arc::new(Mutex::new(Vec::new()));
        let global_payloads_cloned = global_payloads.clone();
        let global_app = axum::Router::new().route(
            "/hook",
            axum::routing::post(move |body: String| {
                let payloads = global_payloads_cloned.clone();
                async move {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&body) {
                        payloads.lock().unwrap().push(value);
                    }
                    axum::http::StatusCode::OK
                }
            }),
        );

        let topic_payloads: Arc<Mutex<Vec<serde_json::Value>>> = Arc::new(Mutex::new(Vec::new()));
        let topic_payloads_cloned = topic_payloads.clone();
        let topic_app = axum::Router::new().route(
            "/hook",
            axum::routing::post(move |body: String| {
                let payloads = topic_payloads_cloned.clone();
                async move {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&body) {
                        payloads.lock().unwrap().push(value);
                    }
                    axum::http::StatusCode::OK
                }
            }),
        );

        let global_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let global_addr = global_listener.local_addr().unwrap();
        let global_server = tokio::spawn(async move {
            axum::serve(global_listener, global_app).await.unwrap();
        });

        let topic_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let topic_addr = topic_listener.local_addr().unwrap();
        let topic_server = tokio::spawn(async move {
            axum::serve(topic_listener, topic_app).await.unwrap();
        });

        let mut config = test_config();
        config.webhook_targets = vec![format!("http://{}/hook", global_addr)];
        let (app, state) = build_router(config).await.expect("build router");

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app.with_state(state)).await.unwrap();
        });

        let endpoint = format!("http://{}", addr);
        let client = reqwest::Client::new();

        let topic_webhook_url = format!("http://{}/hook", topic_addr);
        let owner_token = register_and_auth_http(&client, &endpoint, "upload-owner").await;
        let topic_resp: serde_json::Value = client
            .post(format!("{endpoint}/api/topic/create"))
            .bearer_auth(&owner_token)
            .json(&serde_json::json!({
                "name": "topic-upload-hook",
                "members": ["upload-owner"],
                "multiple": true,
                "webhooks": [topic_webhook_url]
            }))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let topic_id = topic_resp
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap()
            .to_string();

        global_payloads.lock().unwrap().clear();
        topic_payloads.lock().unwrap().clear();

        let boundary = "UPLOAD-HOOK-BOUNDARY";
        let mut upload_body = Vec::new();
        upload_body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"hello.txt\"\r\nContent-Type: text/plain\r\n\r\nhello upload\r\n"
            )
            .as_bytes(),
        );
        upload_body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"topicid\"\r\n\r\n{topic_id}\r\n--{boundary}--\r\n"
            )
            .as_bytes(),
        );
        let upload_resp = client
            .post(format!("{endpoint}/api/attachment/upload"))
            .bearer_auth(&owner_token)
            .header(
                "content-type",
                format!("multipart/form-data; boundary={boundary}"),
            )
            .body(upload_body)
            .send()
            .await
            .unwrap();
        assert_eq!(upload_resp.status(), reqwest::StatusCode::OK);

        for _ in 0..50 {
            if topic_payloads
                .lock()
                .unwrap()
                .iter()
                .any(|payload| payload.get("name").and_then(|n| n.as_str()) == Some("upload.file"))
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        let topic_events = topic_payloads.lock().unwrap().clone();
        let upload_event = topic_events
            .iter()
            .find(|v| v.get("name").and_then(|n| n.as_str()) == Some("upload.file"))
            .cloned()
            .expect("upload.file webhook not found");
        assert_eq!(
            upload_event.get("topicId").and_then(|v| v.as_str()),
            Some(topic_id.as_str())
        );
        assert_eq!(
            upload_event
                .get("data")
                .and_then(|d| d.get("userId"))
                .and_then(|v| v.as_str()),
            Some("upload-owner")
        );
        assert_eq!(
            upload_event
                .get("data")
                .and_then(|d| d.get("data"))
                .and_then(|d| d.get("fileName"))
                .and_then(|v| v.as_str()),
            Some("hello.txt")
        );

        global_payloads.lock().unwrap().clear();

        let guest_resp = client
            .post(format!("{endpoint}/api/guest/login"))
            .header("content-type", "application/json")
            .body(r#"{"guestId":"guest-webhook-user","remember":true}"#)
            .send()
            .await
            .unwrap();
        assert_eq!(guest_resp.status(), reqwest::StatusCode::OK);

        for _ in 0..50 {
            if global_payloads.lock().unwrap().iter().any(|payload| {
                payload.get("name").and_then(|n| n.as_str()) == Some("user.guest.create")
            }) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        let global_events = global_payloads.lock().unwrap().clone();
        let guest_event = global_events
            .iter()
            .find(|v| v.get("name").and_then(|n| n.as_str()) == Some("user.guest.create"))
            .cloned()
            .expect("user.guest.create webhook not found");
        assert_eq!(guest_event.get("topicId").and_then(|v| v.as_str()), None);
        assert_eq!(
            guest_event
                .get("data")
                .and_then(|d| d.get("userId"))
                .and_then(|v| v.as_str()),
            Some("guest-webhook-user")
        );

        server.abort();
        global_server.abort();
        topic_server.abort();
    }

    #[tokio::test]
    async fn webhook_conversation_removed_uses_global_targets_only() {
        use std::sync::{Arc, Mutex};

        let global_payloads: Arc<Mutex<Vec<serde_json::Value>>> = Arc::new(Mutex::new(Vec::new()));
        let global_payloads_cloned = global_payloads.clone();
        let global_app = axum::Router::new().route(
            "/hook",
            axum::routing::post(move |body: String| {
                let payloads = global_payloads_cloned.clone();
                async move {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&body) {
                        payloads.lock().unwrap().push(value);
                    }
                    axum::http::StatusCode::OK
                }
            }),
        );

        let topic_payloads: Arc<Mutex<Vec<serde_json::Value>>> = Arc::new(Mutex::new(Vec::new()));
        let topic_payloads_cloned = topic_payloads.clone();
        let topic_app = axum::Router::new().route(
            "/hook",
            axum::routing::post(move |body: String| {
                let payloads = topic_payloads_cloned.clone();
                async move {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&body) {
                        payloads.lock().unwrap().push(value);
                    }
                    axum::http::StatusCode::OK
                }
            }),
        );

        let global_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let global_addr = global_listener.local_addr().unwrap();
        let global_server = tokio::spawn(async move {
            axum::serve(global_listener, global_app).await.unwrap();
        });

        let topic_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let topic_addr = topic_listener.local_addr().unwrap();
        let topic_server = tokio::spawn(async move {
            axum::serve(topic_listener, topic_app).await.unwrap();
        });

        let mut config = test_config();
        config.webhook_targets = vec![format!("http://{}/hook", global_addr)];
        let (app, state) = build_router(config).await.expect("build router");

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app.with_state(state)).await.unwrap();
        });

        let endpoint = format!("http://{}", addr);
        let client = reqwest::Client::new();

        let owner_token = register_and_auth_http(&client, &endpoint, "conv-rm-a").await;
        let _other_token = register_and_auth_http(&client, &endpoint, "conv-rm-b").await;

        let topic_webhook_url = format!("http://{}/hook", topic_addr);
        let topic_resp: serde_json::Value = client
            .post(format!("{endpoint}/open/topic/create/topic-conv-rm-hook"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({
                "senderId": "conv-rm-a",
                "members": ["conv-rm-a", "conv-rm-b"],
                "multiple": true,
                "webhooks": [topic_webhook_url]
            }))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let topic_id = topic_resp
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap()
            .to_string();

        let send_resp = client
            .post(format!("{endpoint}/open/topic/send/{topic_id}"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({
                "senderId": "conv-rm-a",
                "type": "chat",
                "chatId": "conv-rm-chat-1",
                "message": "seed remove"
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(send_resp.status(), reqwest::StatusCode::OK);

        global_payloads.lock().unwrap().clear();
        topic_payloads.lock().unwrap().clear();

        let remove_resp = client
            .post(format!("{endpoint}/api/chat/remove/{topic_id}"))
            .bearer_auth(&owner_token)
            .send()
            .await
            .unwrap();
        assert_eq!(remove_resp.status(), reqwest::StatusCode::OK);

        for _ in 0..50 {
            if global_payloads.lock().unwrap().iter().any(|payload| {
                payload.get("name").and_then(|n| n.as_str()) == Some("conversation.removed")
            }) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        let global_events = global_payloads.lock().unwrap().clone();
        let removed_event = global_events
            .iter()
            .find(|v| v.get("name").and_then(|n| n.as_str()) == Some("conversation.removed"))
            .cloned()
            .expect("conversation.removed webhook not found in global targets");

        assert_eq!(
            removed_event
                .get("data")
                .and_then(|d| d.get("ownerId"))
                .and_then(|v| v.as_str()),
            Some("conv-rm-a")
        );
        assert_eq!(
            removed_event
                .get("data")
                .and_then(|d| d.get("topicId"))
                .and_then(|v| v.as_str()),
            Some(topic_id.as_str())
        );
        assert_eq!(
            removed_event
                .get("data")
                .and_then(|d| d.get("source"))
                .and_then(|v| v.as_str()),
            Some("api")
        );

        let topic_events = topic_payloads.lock().unwrap().clone();
        assert!(
            !topic_events.iter().any(|v| {
                v.get("name").and_then(|n| n.as_str()) == Some("conversation.removed")
            }),
            "conversation.removed should not be delivered to topic webhooks"
        );

        server.abort();
        global_server.abort();
        topic_server.abort();
    }

    #[tokio::test]
    async fn presence_db_backend_supports_cross_node_snapshot() {
        use crate::infra::presence::{DbPresenceStore, PresenceHub};
        use sea_orm::Database;

        let db_url = format!(
            "sqlite:file:presence-cluster-{}?mode=memory&cache=shared",
            Uuid::new_v4().simple()
        );
        let db = Database::connect(&db_url).await.expect("connect sqlite");
        crate::infra::db::run_migrations(&db)
            .await
            .expect("run migrations");

        let hub_a = PresenceHub::new(std::sync::Arc::new(DbPresenceStore::new(
            db.clone(),
            "node-a".to_string(),
            "127.0.0.1:7001".to_string(),
            90,
        )));
        let hub_b = PresenceHub::new(std::sync::Arc::new(DbPresenceStore::new(
            db,
            "node-b".to_string(),
            "127.0.0.1:7002".to_string(),
            90,
        )));

        hub_a.upsert_session("cluster-user", "ios:a").await;
        let snap_b = hub_b.snapshot("cluster-user").await;
        assert!(snap_b.online);
        assert!(snap_b.devices.iter().any(|v| v == "ios:a"));

        hub_b.upsert_session("cluster-user", "android:b").await;
        let snap_a = hub_a.snapshot("cluster-user").await;
        assert!(snap_a.online);
        assert!(snap_a.devices.iter().any(|v| v == "ios:a"));
        assert!(snap_a.devices.iter().any(|v| v == "android:b"));

        hub_a.remove_session("cluster-user", "ios:a").await;
        hub_b.remove_session("cluster-user", "android:b").await;
        let cleared = hub_a.snapshot("cluster-user").await;
        assert!(!cleared.online);
        assert!(cleared.devices.is_empty());
    }

    #[tokio::test]
    async fn presence_db_backend_persists_endpoint() {
        use crate::infra::presence::{DbPresenceStore, PresenceHub};
        use sea_orm::Database;

        let db_url = format!(
            "sqlite:file:presence-endpoint-{}?mode=memory&cache=shared",
            Uuid::new_v4().simple()
        );
        let db = Database::connect(&db_url).await.expect("connect sqlite");
        crate::infra::db::run_migrations(&db)
            .await
            .expect("run migrations");

        let hub = PresenceHub::new(std::sync::Arc::new(DbPresenceStore::new(
            db.clone(),
            "node-a".to_string(),
            "127.0.0.1:7001".to_string(),
            90,
        )));
        hub.upsert_session("cluster-user", "ios:a").await;

        let row = crate::entity::presence_session::Entity::find_by_id((
            "cluster-user".to_string(),
            "ios:a".to_string(),
        ))
        .one(&db)
        .await
        .unwrap()
        .unwrap();
        assert_eq!(row.node_id, "node-a");
        assert_eq!(row.endpoint, "127.0.0.1:7001");
    }

    #[tokio::test]
    async fn cluster_push_forwards_to_remote_node_via_openapi_push() {
        let db_url = format!(
            "sqlite:file:cluster-push-{}?mode=memory&cache=shared",
            Uuid::new_v4().simple()
        );

        let mut config_a = test_config();
        config_a.database_url = db_url.clone();
        config_a.presence_backend = "db".to_string();
        config_a.presence_node_id = "node-a".to_string();
        config_a.run_migrations = true;

        let mut config_b = test_config();
        config_b.database_url = db_url;
        config_b.presence_backend = "db".to_string();
        config_b.presence_node_id = "node-b".to_string();
        config_b.run_migrations = false;

        let listener_b = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr_b = listener_b.local_addr().unwrap();
        config_b.addr = addr_b.to_string();
        config_b.endpoint = addr_b.to_string();
        let (app_b, state_b) = build_router(config_b).await.expect("rebuild router b");
        let server_b = tokio::spawn(async move {
            axum::serve(listener_b, app_b.with_state(state_b))
                .await
                .unwrap();
        });

        config_a.endpoint = "127.0.0.1:65531".to_string();
        let (app_a, state_a) = build_router(config_a).await.expect("build router a");
        let listener_a = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr_a = listener_a.local_addr().unwrap();
        let server_a = tokio::spawn(async move {
            axum::serve(listener_a, app_a.with_state(state_a))
                .await
                .unwrap();
        });

        let endpoint_a = format!("http://{}", addr_a);
        let endpoint_b = format!("http://{}", addr_b);
        let client = reqwest::Client::new();

        let _ = client
            .post(format!("{endpoint_a}/open/user/register/cluster-a"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();
        let _ = client
            .post(format!("{endpoint_a}/open/user/register/cluster-b"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();

        let bob_auth: serde_json::Value = client
            .post(format!("{endpoint_b}/open/user/auth/cluster-b"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let bob_token = bob_auth.get("authToken").and_then(|v| v.as_str()).unwrap();

        let mut bob_req = format!("ws://{}/api/connect?device=remote-b", addr_b)
            .into_client_request()
            .unwrap();
        bob_req.headers_mut().insert(
            "Authorization",
            format!("Bearer {bob_token}").parse().unwrap(),
        );
        let (mut bob_ws, _) = tokio_tungstenite::connect_async(bob_req).await.unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let send_resp: serde_json::Value = client
            .post(format!("{endpoint_a}/open/chat/cluster-a"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({
                "userIds": ["cluster-b"],
                "chatId": "cluster-msg-1",
                "message": "hello remote node",
                "type": "chat"
            }))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(send_resp.as_array().map(|v| v.len()), Some(1));

        let bob_msg = tokio::time::timeout(std::time::Duration::from_secs(3), bob_ws.next())
            .await
            .expect("remote ws recv timeout")
            .expect("remote ws ended")
            .expect("remote ws error")
            .into_text()
            .unwrap();
        let bob_json: serde_json::Value = serde_json::from_str(&bob_msg).unwrap();
        assert_eq!(
            bob_json.get("chatId").and_then(|v| v.as_str()),
            Some("cluster-msg-1")
        );

        server_a.abort();
        server_b.abort();
    }

    struct ClusterSystemPushFixture {
        endpoint_a: String,
        endpoint_b: String,
        addr_b: std::net::SocketAddr,
        server_a: tokio::task::JoinHandle<()>,
        server_b: tokio::task::JoinHandle<()>,
        client: reqwest::Client,
    }

    impl ClusterSystemPushFixture {
        async fn start(name: &str) -> Self {
            let db_url = format!(
                "sqlite:file:{name}-{}?mode=memory&cache=shared",
                Uuid::new_v4().simple()
            );

            let mut config_a = test_config();
            config_a.database_url = db_url.clone();
            config_a.presence_backend = "db".to_string();
            config_a.presence_node_id = format!("{name}-node-a");
            config_a.run_migrations = true;

            let mut config_b = test_config();
            config_b.database_url = db_url;
            config_b.presence_backend = "db".to_string();
            config_b.presence_node_id = format!("{name}-node-b");
            config_b.run_migrations = false;

            let listener_b = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr_b = listener_b.local_addr().unwrap();
            config_b.addr = addr_b.to_string();
            config_b.endpoint = addr_b.to_string();
            let (app_b, state_b) = build_router(config_b).await.expect("build router b");
            let server_b = tokio::spawn(async move {
                axum::serve(listener_b, app_b.with_state(state_b))
                    .await
                    .unwrap();
            });

            config_a.endpoint = "127.0.0.1:65531".to_string();
            let (app_a, state_a) = build_router(config_a).await.expect("build router a");
            let listener_a = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr_a = listener_a.local_addr().unwrap();
            let server_a = tokio::spawn(async move {
                axum::serve(listener_a, app_a.with_state(state_a))
                    .await
                    .unwrap();
            });

            Self {
                endpoint_a: format!("http://{}", addr_a),
                endpoint_b: format!("http://{}", addr_b),
                addr_b,
                server_a,
                server_b,
                client: reqwest::Client::new(),
            }
        }

        async fn register_pair(&self, owner_id: &str, other_id: &str) {
            let _ = self
                .client
                .post(format!("{}/open/user/register/{owner_id}", self.endpoint_a))
                .bearer_auth("test-token")
                .json(&serde_json::json!({}))
                .send()
                .await
                .unwrap();
            let _ = self
                .client
                .post(format!("{}/open/user/register/{other_id}", self.endpoint_a))
                .bearer_auth("test-token")
                .json(&serde_json::json!({}))
                .send()
                .await
                .unwrap();
        }

        async fn connect_remote_ws(
            &self,
            user_id: &str,
            device: &str,
        ) -> tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        > {
            let auth: serde_json::Value = self
                .client
                .post(format!("{}/open/user/auth/{user_id}", self.endpoint_b))
                .bearer_auth("test-token")
                .json(&serde_json::json!({}))
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
            let token = auth.get("authToken").and_then(|v| v.as_str()).unwrap();

            let mut req = format!("ws://{}/api/connect?device={device}", self.addr_b)
                .into_client_request()
                .unwrap();
            req.headers_mut().insert(
                "Authorization",
                format!("Bearer {token}").parse().unwrap(),
            );
            let (ws, _) = tokio_tungstenite::connect_async(req).await.unwrap();
            ws
        }

        async fn create_topic(&self, owner_id: &str, other_id: &str, topic_id: &str) {
            let resp = self
                .client
                .post(format!("{}/open/topic/create/{topic_id}", self.endpoint_a))
                .bearer_auth("test-token")
                .json(&serde_json::json!({
                    "senderId": owner_id,
                    "members": [other_id],
                    "multiple": true,
                    "ensureConversation": true,
                }))
                .send()
                .await
                .unwrap();
            assert_eq!(resp.status(), reqwest::StatusCode::OK);
        }

        async fn shutdown(self) {
            self.server_a.abort();
            self.server_b.abort();
        }
    }

    #[tokio::test]
    async fn cluster_conversation_update_forwards_system_chat_to_remote_node() {
        let fixture = ClusterSystemPushFixture::start("cluster-conv-update").await;
        fixture.register_pair("cluster-cu-a", "cluster-cu-b").await;
        fixture
            .create_topic("cluster-cu-a", "cluster-cu-b", "cluster-conv-update-topic")
            .await;

        let mut remote_ws = fixture.connect_remote_ws("cluster-cu-a", "remote-owner").await;
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let resp = fixture
            .client
            .post(format!(
                "{}/open/conversation/update/cluster-cu-a/cluster-conv-update-topic",
                fixture.endpoint_a
            ))
            .bearer_auth("test-token")
            .json(&serde_json::json!({"sticky": true, "remark": "cluster pin"}))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), reqwest::StatusCode::OK);

        let pushed = tokio::time::timeout(std::time::Duration::from_secs(3), remote_ws.next())
            .await
            .expect("remote update push timeout")
            .expect("remote update stream ended")
            .expect("remote update ws error")
            .into_text()
            .unwrap();
        let pushed_json: serde_json::Value = serde_json::from_str(&pushed).unwrap();
        assert_eq!(
            pushed_json
                .get("content")
                .and_then(|v| v.get("type"))
                .and_then(|v| v.as_str()),
            Some("conversation.update")
        );
        let fields: serde_json::Value = serde_json::from_str(
            pushed_json
                .get("content")
                .and_then(|v| v.get("text"))
                .and_then(|v| v.as_str())
                .unwrap(),
        )
        .unwrap();
        assert_eq!(fields.get("sticky").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(
            fields.get("remark").and_then(|v| v.as_str()),
            Some("cluster pin")
        );

        let _ = remote_ws.close(None).await;
        fixture.shutdown().await;
    }

    #[tokio::test]
    async fn cluster_mark_unread_forwards_system_chat_to_remote_node() {
        let fixture = ClusterSystemPushFixture::start("cluster-mark-unread").await;
        fixture.register_pair("cluster-mu-a", "cluster-mu-b").await;
        fixture
            .create_topic("cluster-mu-a", "cluster-mu-b", "cluster-mark-unread-topic")
            .await;

        let mut remote_ws = fixture.connect_remote_ws("cluster-mu-a", "remote-owner").await;
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let resp = fixture
            .client
            .post(format!(
                "{}/open/conversation/unread/cluster-mu-a/cluster-mark-unread-topic",
                fixture.endpoint_a
            ))
            .bearer_auth("test-token")
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), reqwest::StatusCode::OK);

        let pushed = tokio::time::timeout(std::time::Duration::from_secs(3), remote_ws.next())
            .await
            .expect("remote unread push timeout")
            .expect("remote unread stream ended")
            .expect("remote unread ws error")
            .into_text()
            .unwrap();
        let pushed_json: serde_json::Value = serde_json::from_str(&pushed).unwrap();
        assert_eq!(
            pushed_json
                .get("content")
                .and_then(|v| v.get("type"))
                .and_then(|v| v.as_str()),
            Some("conversation.update")
        );
        let fields: serde_json::Value = serde_json::from_str(
            pushed_json
                .get("content")
                .and_then(|v| v.get("text"))
                .and_then(|v| v.as_str())
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            fields.get("markUnread").and_then(|v| v.as_bool()),
            Some(true)
        );

        let _ = remote_ws.close(None).await;
        fixture.shutdown().await;
    }

    #[tokio::test]
    async fn cluster_conversation_removed_forwards_system_chat_to_remote_node() {
        let fixture = ClusterSystemPushFixture::start("cluster-conv-removed").await;
        fixture.register_pair("cluster-cr-a", "cluster-cr-b").await;
        fixture
            .create_topic("cluster-cr-a", "cluster-cr-b", "cluster-conv-removed-topic")
            .await;

        let mut remote_ws = fixture.connect_remote_ws("cluster-cr-a", "remote-owner").await;
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let resp = fixture
            .client
            .post(format!(
                "{}/open/conversation/remove/cluster-cr-a/cluster-conv-removed-topic",
                fixture.endpoint_a
            ))
            .bearer_auth("test-token")
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), reqwest::StatusCode::OK);

        let pushed = tokio::time::timeout(std::time::Duration::from_secs(3), remote_ws.next())
            .await
            .expect("remote removed push timeout")
            .expect("remote removed stream ended")
            .expect("remote removed ws error")
            .into_text()
            .unwrap();
        let pushed_json: serde_json::Value = serde_json::from_str(&pushed).unwrap();
        assert_eq!(
            pushed_json
                .get("content")
                .and_then(|v| v.get("type"))
                .and_then(|v| v.as_str()),
            Some("conversation.removed")
        );
        assert_eq!(
            pushed_json.get("topicId").and_then(|v| v.as_str()),
            Some("cluster-conv-removed-topic")
        );

        let _ = remote_ws.close(None).await;
        fixture.shutdown().await;
    }

    #[tokio::test]
    async fn admin_bootstrap_creates_first_superuser_once() {
        let (app, state) = build_router(test_config()).await.expect("build router");
        let app = app.with_state(state.clone());

        let bootstrap_req = Request::builder()
            .uri("/admin/api/bootstrap")
            .method("GET")
            .body(Body::empty())
            .unwrap();
        let bootstrap_resp = app.clone().oneshot(bootstrap_req).await.unwrap();
        assert_eq!(bootstrap_resp.status(), StatusCode::OK);
        let bootstrap_body = bootstrap_resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let bootstrap_json: serde_json::Value = serde_json::from_slice(&bootstrap_body).unwrap();
        assert_eq!(
            bootstrap_json
                .get("superuserCount")
                .and_then(|v| v.as_u64()),
            Some(0)
        );

        let init_req = Request::builder()
            .uri("/admin/api/bootstrap")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"userId":"root-admin","displayName":"Root","password":"secret-123"}"#,
            ))
            .unwrap();
        let init_resp = app.clone().oneshot(init_req).await.unwrap();
        assert_eq!(init_resp.status(), StatusCode::OK);
        let init_body = init_resp.into_body().collect().await.unwrap().to_bytes();
        let init_json: serde_json::Value = serde_json::from_slice(&init_body).unwrap();
        assert_eq!(
            init_json.get("userId").and_then(|v| v.as_str()),
            Some("root-admin")
        );
        assert!(init_json.get("token").and_then(|v| v.as_str()).is_some());

        let model = crate::entity::user::Entity::find_by_id("root-admin".to_string())
            .one(&state.db)
            .await
            .unwrap()
            .unwrap();
        assert!(model.is_staff);
        assert!(model.enabled);
        assert_ne!(model.password, "secret-123");

        let second_init_req = Request::builder()
            .uri("/admin/api/bootstrap")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"userId":"root-admin-2","password":"secret-456"}"#,
            ))
            .unwrap();
        let second_init_resp = app.oneshot(second_init_req).await.unwrap();
        assert_eq!(second_init_resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn openapi_user_and_topic_list_and_enable_controls_work() {
        let (app, state) = build_router(test_config()).await.expect("build router");
        let app = app.with_state(state);

        let _ = register_and_auth(&app, "admin-user").await;
        let staff_req = Request::builder()
            .uri("/open/user/staff/admin-user")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"isStaff":true}"#))
            .unwrap();
        let staff_resp = app.clone().oneshot(staff_req).await.unwrap();
        assert_eq!(staff_resp.status(), StatusCode::OK);

        let _ = register_and_auth(&app, "member-a").await;
        let topic_create_req = Request::builder()
            .uri("/open/topic/create/topic-admin-list")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"senderId":"admin-user","members":["admin-user","member-a"],"multiple":true}"#,
            ))
            .unwrap();
        let topic_create_resp = app.clone().oneshot(topic_create_req).await.unwrap();
        assert_eq!(topic_create_resp.status(), StatusCode::OK);

        let user_list_req = Request::builder()
            .uri("/open/user/list")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"keyword":"member","offset":0,"limit":10}"#))
            .unwrap();
        let user_list_resp = app.clone().oneshot(user_list_req).await.unwrap();
        assert_eq!(user_list_resp.status(), StatusCode::OK);
        let user_list_body = user_list_resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let user_list_json: serde_json::Value = serde_json::from_slice(&user_list_body).unwrap();
        assert_eq!(
            user_list_json.get("total").and_then(|v| v.as_u64()),
            Some(1)
        );

        let disable_user_req = Request::builder()
            .uri("/open/user/enabled/member-a")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"enabled":false}"#))
            .unwrap();
        let disable_user_resp = app.clone().oneshot(disable_user_req).await.unwrap();
        assert_eq!(disable_user_resp.status(), StatusCode::OK);

        let topic_list_req = Request::builder()
            .uri("/open/topic/list")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"keyword":"topic-admin-list","offset":0,"limit":10}"#,
            ))
            .unwrap();
        let topic_list_resp = app.clone().oneshot(topic_list_req).await.unwrap();
        assert_eq!(topic_list_resp.status(), StatusCode::OK);
        let topic_list_body = topic_list_resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let topic_list_json: serde_json::Value = serde_json::from_slice(&topic_list_body).unwrap();
        assert_eq!(
            topic_list_json.get("total").and_then(|v| v.as_u64()),
            Some(1)
        );

        let disable_topic_req = Request::builder()
            .uri("/open/topic/enabled/topic-admin-list")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"enabled":false}"#))
            .unwrap();
        let disable_topic_resp = app.clone().oneshot(disable_topic_req).await.unwrap();
        assert_eq!(disable_topic_resp.status(), StatusCode::OK);

        let send_req = Request::builder()
            .uri("/open/topic/send/topic-admin-list")
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"senderId":"admin-user","type":"chat","message":"hello after disable"}"#,
            ))
            .unwrap();
        let send_resp = app.oneshot(send_req).await.unwrap();
        assert_eq!(send_resp.status(), StatusCode::UNAUTHORIZED);
    }

    fn extract_token(json: &str) -> Option<String> {
        let v: serde_json::Value = serde_json::from_str(json).ok()?;
        v.get("authToken")?.as_str().map(str::to_string)
    }

    async fn register_and_auth<S>(app: &S, user_id: &str) -> String
    where
        S: tower::Service<
                Request<Body>,
                Response = axum::response::Response,
                Error = std::convert::Infallible,
            > + Clone,
        S::Future: Send,
    {
        let reg_req = Request::builder()
            .uri(format!("/open/user/register/{user_id}"))
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .unwrap();
        let _ = app.clone().oneshot(reg_req).await.unwrap();

        let auth_req = Request::builder()
            .uri(format!("/open/user/auth/{user_id}"))
            .method("POST")
            .header("Authorization", "Bearer test-token")
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .unwrap();
        let auth_resp = app.clone().oneshot(auth_req).await.unwrap();
        let body = auth_resp.into_body().collect().await.unwrap().to_bytes();
        let text = String::from_utf8(body.to_vec()).unwrap();
        extract_token(&text).expect("extract user token")
    }

    async fn register_and_auth_http(
        client: &reqwest::Client,
        endpoint: &str,
        user_id: &str,
    ) -> String {
        let _ = client
            .post(format!("{endpoint}/open/user/register/{user_id}"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();

        let auth_resp: serde_json::Value = client
            .post(format!("{endpoint}/open/user/auth/{user_id}"))
            .bearer_auth("test-token")
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

        auth_resp
            .get("authToken")
            .and_then(|v| v.as_str())
            .unwrap()
            .to_string()
    }
}
