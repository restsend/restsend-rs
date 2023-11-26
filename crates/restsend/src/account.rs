use crate::models::AuthInfo;
use anyhow::Result;

pub fn set_current_user(root: &str, user_id: &str) -> Result<()> {
    let current_file = std::path::PathBuf::from(root).join(".current_user");
    if user_id != String::default() {
        let user_dir = std::path::PathBuf::from(root).join(&user_id);
        std::fs::create_dir_all(std::path::Path::new(&user_dir))?;
    }
    Ok(std::fs::write(&current_file, user_id)?)
}

pub fn get_current_user(root: &str) -> Option<AuthInfo> {
    let current_file = std::path::PathBuf::from(root).join(".current_user");
    match std::fs::read_to_string(&current_file) {
        Ok(user_id) => Some(AuthInfo {
            user_id,
            ..Default::default()
        }),
        _ => None,
    }
}

#[test]
fn test_get_current_user() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_str().unwrap();
    let user = get_current_user(root);
    assert!(user.is_none());

    set_current_user(root, "hello").expect("set current user failed");
    let user = get_current_user(root);
    assert!(user.is_some());
}
