use crate::{
    client::{tests::TEST_ENDPOINT, Client},
    services::auth::login_with_password,
    utils::init_log,
};

#[tokio::test]
async fn test_get_user() {
    init_log("INFO".to_string(), true);
    let info = login_with_password(
        TEST_ENDPOINT.to_string(),
        "guido".to_string(),
        "guido:demo".to_string(),
    )
    .await;
    let c = Client::new("".to_string(), "".to_string(), &info.unwrap());

    let guido = c.get_user("guido".to_string(), false).await;
    match guido {
        Some(u) => {
            assert_eq!(u.user_id, "guido");
            assert_eq!(u.is_partial, true);
        }
        None => {
            assert!(false);
        }
    }
    let guido = c.get_user("guido".to_string(), true).await;
    match guido {
        Some(u) => {
            assert_eq!(u.user_id, "guido");
            assert_eq!(u.is_partial, false);
        }
        None => {
            assert!(false);
        }
    }
}
