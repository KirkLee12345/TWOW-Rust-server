use serde::{Deserialize, Serialize};
const USER_INFO_DB: &str = "user_info";
const USER_DATA_DB: &str = "user_data";


#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct UserInfo {
    pub password_hash: String,
    pub email: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct UserData {
    pub money: u128,
}

pub fn is_email_exist(email: &str) -> anyhow::Result<bool> {
    let db = sled::open(USER_INFO_DB)?;
    let key = email.as_bytes();
    match db.get(key)? {
        Some(_) => Ok(true),
        None => Ok(false),
    }
}

pub fn is_user_exist(username: &str) -> anyhow::Result<bool> {
    let db = sled::open(USER_INFO_DB)?;
    let key = username.as_bytes();
    match db.get(key)? {
        Some(_) => Ok(true),
        None => Ok(false),
    }
}

pub fn save_user_info(username: &str, user_info: &UserInfo) -> anyhow::Result<()> {
    let db = sled::open(USER_INFO_DB)?;
    let key = username.as_bytes();
    let value = serde_json::to_vec(user_info)?;
    db.insert(key, value)?;
    Ok(())
}

pub fn load_user_info(username: &str) -> anyhow::Result<Option<UserInfo>> {
    let db = sled::open(USER_INFO_DB)?;
    let key = username.as_bytes();
    match db.get(key)? {
        Some(bytes) => {
            let user_info = serde_json::from_slice(&bytes)?;
            Ok(Some(user_info))
        }
        None => Ok(None),
    }
}

pub fn save_user_data(username: &str, user_data: &UserData) -> anyhow::Result<()> {
    let db = sled::open(USER_DATA_DB)?;
    let key = username.as_bytes();
    let value = serde_json::to_vec(user_data)?;
    db.insert(key, value)?;
    Ok(())
}

pub fn load_user_data(username: &str) -> anyhow::Result<Option<UserData>> {
    let db = sled::open(USER_DATA_DB)?;
    let key = username.as_bytes();
    match db.get(key)? {
        Some(bytes) => {
            let user_data = serde_json::from_slice(&bytes)?;
            Ok(Some(user_data))
        }
        None => Ok(None),
    }
}
