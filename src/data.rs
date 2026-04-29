use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Users(pub HashMap<String, UserRecord>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRecord {
    pub password_hash: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserData(pub HashMap<String, UserDataRecord>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDataRecord {
    pub money: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmailCodes(pub HashMap<String, String>);

pub struct DataManager;

impl DataManager {
    pub fn load_users() -> Users {
        if !Path::new("users.json").exists() {
            return Users::default();
        }
        let content = fs::read_to_string("users.json").unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    }

    pub fn save_users(users: &Users) {
        let content = serde_json::to_string_pretty(users).unwrap();
        fs::write("users.json", content).ok();
    }

    pub fn load_user_data() -> UserData {
        if !Path::new("user_data.json").exists() {
            return UserData::default();
        }
        let content = fs::read_to_string("user_data.json").unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    }

    pub fn save_user_data(data: &UserData) {
        let content = serde_json::to_string_pretty(data).unwrap();
        fs::write("user_data.json", content).ok();
    }

    pub fn load_email_key() -> String {
        if !Path::new("email.key").exists() {
            fs::write("email.key", "xxxxxxxxxxxxxxxx").ok();
            println!("Warning: Email key not configured, please edit email.key");
            String::new()
        } else {
            fs::read_to_string("email.key").unwrap_or_default().trim().to_string()
        }
    }
}