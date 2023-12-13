use crate::client::Client;

#[cfg(not(target_family = "wasm"))]
#[test]
fn test_db_prepare() {
    //
    let d = crate::models::DBStore::new(crate::models::MEMORY_DSN);
    assert!(d.prepare().is_ok());
}

#[cfg(not(target_family = "wasm"))]
#[test]
fn test_client_init() {
    let c = Client::new(crate::models::MEMORY_DSN.to_string(), "".to_string());
    c.prepare().expect("prepare failed");
}
#[test]
fn test_prepare_demo_users() {
    super::signup_demo_user("alice").expect("signup alice failed");
    super::signup_demo_user("bob").expect("signup bob failed");
    super::signup_demo_user("guido").expect("signup guido failed");
    super::signup_demo_user("vitalik").expect("signup vitalik failed");
    super::signup_demo_user("carol").expect("signup carol failed");
}

#[test]
fn test_bad_ssl_certificate() {
    crate::init_log(String::from("debug"), true);
    let c = Client::new(
        crate::models::MEMORY_DSN.to_string(),
        "https://chat.ruzhila.cn".to_string(),
    );
    assert!(c.prepare().is_ok());
    let _ = crate::login(
        "https://chat.rddoc.cn".to_string(),
        "alice".to_string(),
        "alice:demo".to_string(),
    )
    .expect_err("invalid peer certificate");
}

#[test]
fn test_login_and_getchatlogs() {
    crate::init_log(String::from("debug"), true);
    let c = super::login_with("guido", "guido:demo");
    let logs = c
        .get_chat_logs_desc("guido:vitalik".to_string(), 0, 0)
        .expect("logs failed");

    assert!(logs.items.len() > 0 || logs.items.len() == 0);
}
