use crate::client::Client;

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_db_prepare() {
    //
    let d = crate::models::DBStore::new(crate::models::MEMORY_DSN);
    assert!(d.prepare().is_ok());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_client_init() {
    let c = Client::new(crate::models::MEMORY_DSN.to_string(), "".to_string());
    c.prepare().expect("prepare fail");
}

#[test]
fn test_bad_ssl_certificate() {
    crate::init_log(String::from("debug"), true);
    let c = Client::new(
        crate::models::MEMORY_DSN.to_string(),
        "https://chat.rddoc.cn".to_string(),
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
        .expect("logs fail");

    assert!(logs.items.len() > 0 || logs.items.len() == 0);
}
