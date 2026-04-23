use crate::{
    client::{
        tests::{test_endpoint, unique_test_user},
        Client,
    },
    services::auth::{login_with_password, signup},
    utils::init_log,
};

#[tokio::test]
async fn test_get_user() {
    init_log("INFO".to_string(), true);

    let user_id = unique_test_user("sdk-user");
    signup(test_endpoint(), user_id.clone(), "pass-1".to_string())
        .await
        .expect("signup user");

    let info = login_with_password(
        test_endpoint(),
        user_id.clone(),
        "pass-1".to_string(),
    )
    .await;
    let c = Client::new("".to_string(), "".to_string(), &info.unwrap());

    let user = c.get_user(user_id.clone(), false).await;
    match user {
        Some(u) => {
            assert_eq!(u.user_id, user_id.as_str());
            assert_eq!(u.is_partial, true);
        }
        None => {
            assert!(false);
        }
    }
    let user = c.get_user(user_id.clone(), true).await;
    match user {
        Some(u) => {
            assert_eq!(u.user_id, user_id.as_str());
            assert_eq!(u.is_partial, false);
        }
        None => {
            assert!(false);
        }
    }
}
