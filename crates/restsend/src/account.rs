use std::path::{Path, PathBuf};

use crate::models::AuthInfo;
use anyhow::Result;
use std::fs::{create_dir_all, read_to_string, write};

pub fn set_current_user(root: &str, user_id: &str) -> Result<()> {
    let current_file = PathBuf::from(root).join(".current_user");
    if user_id != String::default() {
        let user_dir = PathBuf::from(root).join(&user_id);
        create_dir_all(Path::new(&user_dir))?;
    }
    Ok(write(&current_file, user_id)?)
}

pub fn get_current_user(root: &str) -> Option<AuthInfo> {
    let current_file = PathBuf::from(root).join(".current_user");
    match read_to_string(&current_file) {
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
