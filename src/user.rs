use crate::data::{UserDataRecord, UserRecord};
use std::collections::HashMap;

fn simple_hash(data: &str) -> String {
    let mut hash: u64 = 0;
    for (i, byte) in data.bytes().enumerate() {
        hash = hash.wrapping_add((byte as u64).wrapping_mul((i + 1) as u64));
        hash = hash.rotate_left(5);
    }
    format!("{:016x}", hash)
}

pub struct OnlineUser {
    pub index: usize,
    pub room: Option<String>,
}

pub struct UserManager {
    pub users: HashMap<String, UserRecord>,
    pub user_data: HashMap<String, UserDataRecord>,
    pub email_codes: HashMap<String, String>,
    pub online_users: HashMap<String, OnlineUser>,
}

impl UserManager {
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
            user_data: HashMap::new(),
            email_codes: HashMap::new(),
            online_users: HashMap::new(),
        }
    }

    pub fn hash_password(password: &str) -> String {
        simple_hash(password)
    }

    pub fn verify_password(&self, username: &str, password: &str) -> bool {
        self.users.get(username)
            .map(|u| u.password_hash == Self::hash_password(password))
            .unwrap_or(false)
    }

    pub fn is_username_taken(&self, username: &str) -> bool {
        self.users.contains_key(username)
    }

    pub fn is_email_taken(&self, email: &str) -> bool {
        self.users.values().any(|u| u.email == email)
    }

    pub fn add_user(&mut self, username: String, password_hash: String, email: String) -> bool {
        if self.users.contains_key(&username) {
            return false;
        }
        if self.users.values().any(|u| u.email == email) {
            return false;
        }
        self.users.insert(username.clone(), UserRecord { password_hash, email });
        self.user_data.insert(username, UserDataRecord { money: 0 });
        true
    }

    pub fn add_email_code(&mut self, email: &str, code: String) {
        self.email_codes.insert(email.to_string(), code);
    }

    pub fn verify_email_code(&mut self, email: &str, code: &str) -> bool {
        self.email_codes.get(email).map(|c| c == code).unwrap_or(false)
    }

    pub fn remove_email_code(&mut self, email: &str) {
        self.email_codes.remove(email);
    }

    pub fn add_online_user(&mut self, username: String, index: usize) {
        self.online_users.insert(username, OnlineUser { index, room: None });
    }

    pub fn remove_online_user(&mut self, username: &str) -> Option<OnlineUser> {
        self.online_users.remove(username)
    }

    pub fn find_user_by_index(&self, index: usize) -> Option<String> {
        self.online_users.iter()
            .find(|(_, u)| u.index == index)
            .map(|(k, _)| k.clone())
    }

    pub fn find_user_index_by_name(&self, username: &str) -> Option<usize> {
        self.online_users.get(username).map(|u| u.index)
    }

    pub fn find_user_in_room(&self, room_name: &str, exclude: &str) -> Option<String> {
        self.online_users.iter()
            .find(|(name, u)| u.room.as_deref() == Some(room_name) && *name != exclude)
            .map(|(k, _)| k.clone())
    }

    pub fn set_user_room(&mut self, username: &str, room: Option<String>) {
        if let Some(user) = self.online_users.get_mut(username) {
            user.room = room;
        }
    }

    pub fn get_user_room(&self, username: &str) -> Option<String> {
        self.online_users.get(username).and_then(|u| u.room.clone())
    }

    pub fn get_money(&self, username: &str) -> i32 {
        self.user_data.get(username).map(|d| d.money).unwrap_or(0)
    }

    pub fn add_money(&mut self, username: &str, amount: i32) {
        if let Some(data) = self.user_data.get_mut(username) {
            data.money += amount;
        }
    }

    pub fn subtract_money(&mut self, username: &str, amount: i32) -> bool {
        if let Some(data) = self.user_data.get_mut(username) {
            if data.money >= amount {
                data.money -= amount;
                return true;
            }
        }
        false
    }
}

impl Default for UserManager {
    fn default() -> Self {
        Self::new()
    }
}