use crate::data::DataManager;
use crate::email::{generate_verification_code, EmailService};
use crate::game::{GameLogic, GameResult};
use crate::room::RoomManager;
use crate::user::UserManager;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub struct Server {
    pub version: String,
    pub protocol_version: i32,
    pub host: String,
    pub port: u16,
    pub user_manager: Mutex<UserManager>,
    pub room_manager: Mutex<RoomManager>,
    pub email_service: Mutex<Option<EmailService>>,
    pub new_index: Mutex<usize>,
    pub debug_mode: Mutex<bool>,
    pub sockets: Mutex<HashMap<usize, TcpStream>>,
}

impl Server {
    pub fn new(host: String, port: u16) -> Self {
        let email_key = DataManager::load_email_key();
        let email_service = if email_key.is_empty() || email_key == "xxxxxxxxxxxxxxxx" {
            None
        } else {
            Some(EmailService::new(
                "TDR_Group@foxmail.com".to_string(),
                email_key,
            ))
        };

        Self {
            version: "V1.2.3".to_string(),
            protocol_version: 3,
            host,
            port,
            user_manager: Mutex::new(UserManager::new()),
            room_manager: Mutex::new(RoomManager::new()),
            email_service: Mutex::new(email_service),
            new_index: Mutex::new(0),
            debug_mode: Mutex::new(true),
            sockets: Mutex::new(HashMap::new()),
        }
    }

    pub fn load(&self) {
        let users = DataManager::load_users();
        let user_data = DataManager::load_user_data();

        let mut um = self.user_manager.lock().unwrap();
        um.users = users.0;
        um.user_data = user_data.0;

        println!("Data loaded: {} users", um.users.len());
    }

    pub fn save(&self) {
        let um = self.user_manager.lock().unwrap();
        DataManager::save_users(&crate::data::Users(um.users.clone()));
        DataManager::save_user_data(&crate::data::UserData(um.user_data.clone()));
    }

    pub fn run(&self) {
        let addr = format!("{}:{}", self.host, self.port);
        let listener = TcpListener::bind(&addr).unwrap();
        listener.set_nonblocking(true).ok();

        println!(
            "Server started, version: {}, protocol: {}, listening on: {}",
            self.version, self.protocol_version, addr
        );

        loop {
            match listener.accept() {
                Ok((socket, addr)) => {
                    println!("New connection from {}", addr);
                    let index = {
                        let mut idx = self.new_index.lock().unwrap();
                        let i = *idx;
                        *idx += 1;
                        i
                    };

                    {
                        let mut sockets = self.sockets.lock().unwrap();
                        sockets.insert(index, socket.try_clone().unwrap());
                    }

                    let server = Arc::new(self.clone_inner());
                    let srv = server.clone();
                    let sock = socket.try_clone().unwrap();
                    thread::spawn(move || {
                        srv.handle_client(sock, index);
                    });
                }
                Err(_) => {
                    thread::sleep(Duration::from_millis(10));
                }
            }
        }
    }

    fn clone_inner(&self) -> Self {
        Self {
            version: self.version.clone(),
            protocol_version: self.protocol_version,
            host: self.host.clone(),
            port: self.port,
            user_manager: Mutex::new(UserManager::new()),
            room_manager: Mutex::new(RoomManager::new()),
            email_service: Mutex::new(None),
            new_index: Mutex::new(0),
            debug_mode: Mutex::new(*self.debug_mode.lock().unwrap()),
            sockets: Mutex::new(HashMap::new()),
        }
    }

    fn handle_client(&self, mut socket: TcpStream, index: usize) {
        socket.set_nonblocking(true).ok();

        loop {
            let mut buf = [0u8; 1024];
            match socket.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let data = String::from_utf8_lossy(&buf[..n]);
                    let parts: Vec<&str> = data.split_whitespace().collect();

                    if parts.is_empty() {
                        continue;
                    }

                    if *self.debug_mode.lock().unwrap() {
                        println!("Received from {}: {}", index, data.trim());
                    }

                    let response = self.process_command(&parts, index);
                    if !response.is_empty() {
                        socket.write_all(response.as_bytes()).ok();
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(_) => break,
            }
        }

        self.handle_disconnect(index);
    }

    fn process_command(&self, parts: &[&str], index: usize) -> String {
        if parts.is_empty() {
            return String::new();
        }

        let cmd = parts[0];

        match cmd {
            "f**k" => {
                if parts.len() >= 2 {
                    return format!("f**k {} ", parts[1]);
                }
                String::new()
            }
            "login" => self.handle_login(parts, index),
            "sign" => self.handle_sign(parts, index),
            "selfinfo" => self.handle_selfinfo(parts, index),
            "room" => self.handle_room(parts, index),
            "game" => self.handle_game(parts, index),
            "test" => self.handle_test(parts, index),
            _ => String::new(),
        }
    }

    fn handle_login(&self, parts: &[&str], index: usize) -> String {
        if parts.len() != 4 {
            return "tip 参数错误(参数数量不是4个) ".to_string();
        }

        let proto_ver: i32 = parts[1].parse().unwrap_or(0);
        if proto_ver != self.protocol_version {
            return format!("loginfail {} ", self.protocol_version);
        }

        let username = parts[2];
        let password = parts[3];

        let um = self.user_manager.lock().unwrap();
        if um.verify_password(username, password) {
            if let Some(mut socket) = self.sockets.lock().unwrap().get(&index) {
                let mut sockets = self.sockets.lock().unwrap();
                sockets.insert(index, socket.try_clone().unwrap());
            }
            drop(um);
            let mut um = self.user_manager.lock().unwrap();
            if um.online_users.contains_key(username) {
                if let Some(mut s) = self.sockets.lock().unwrap().get(&index) {
                    s.write_all("重复登陆! ".as_bytes()).ok();
                }
            }
            um.add_online_user(username.to_string(), index);
            println!("User {} logged in", username);
            "登陆成功! ".to_string()
        } else {
            "账号密码错误! ".to_string()
        }
    }

    fn handle_sign(&self, parts: &[&str], _index: usize) -> String {
        if parts.len() == 3 {
            if parts[1] == "username" {
                let um = self.user_manager.lock().unwrap();
                return if um.is_username_taken(parts[2]) {
                    "用户名不可用".to_string()
                } else {
                    "*用户名可用*".to_string()
                };
            }
            if parts[1] == "ema" {
                let email = parts[2];
                let es = self.email_service.lock().unwrap();
                if let Some(service) = es.as_ref() {
                    let code = generate_verification_code();
                    {
                        let mut um = self.user_manager.lock().unwrap();
                        um.add_email_code(email, code.clone());
                    }
                    if service.send_verification(email, &code).is_ok() {
                        println!("Verification code {} sent to {}", code, email);
                        return "sand yzm sucess ".to_string();
                    }
                }
            }
        }

        if parts.len() == 6 && parts[1] == "up" {
            let username = parts[2];
            let password = parts[3];
            let email = parts[4];
            let code = parts[5];

            let mut um = self.user_manager.lock().unwrap();
            if um.is_username_taken(username) {
                return "用户名不可用".to_string();
            }
            if um.is_email_taken(email) {
                return "此邮箱已被绑定!".to_string();
            }

            if um.verify_email_code(email, code) {
                um.remove_email_code(email);
                let password_hash = UserManager::hash_password(password);
                if um.add_user(username.to_string(), password_hash, email.to_string()) {
                    drop(um);
                    self.save();
                    println!("User {} registered with email {}", username, email);
                    return "注册成功! ".to_string();
                }
            }
            return "验证码错误! ".to_string();
        }

        "tip 参数错误(参数数量不对) ".to_string()
    }

    fn handle_selfinfo(&self, parts: &[&str], index: usize) -> String {
        let um = self.user_manager.lock().unwrap();
        if let Some(username) = um.find_user_by_index(index) {
            let money = um.get_money(&username);
            let online_count = um.online_users.len();
            return format!("selfinfo {} {} {} ", username, money, online_count);
        }
        "tip 参数错误(未登录) ".to_string()
    }

    fn handle_room(&self, parts: &[&str], index: usize) -> String {
        let um = self.user_manager.lock().unwrap();
        let username = match um.find_user_by_index(index) {
            Some(u) => u,
            None => return "tip 参数错误(未登录) ".to_string(),
        };
        drop(um);

        if parts.len() == 3 {
            if parts[1] == "create" {
                return self.room_create(&username, parts[2]);
            }
            if parts[1] == "join" {
                return self.room_join(&username, parts[2], index);
            }
        }

        if parts.len() == 2 {
            if parts[1] == "r" {
                return self.room_list();
            }
            if parts[1] == "exit" {
                return self.room_exit(&username);
            }
        }

        "tip 参数错误(参数数量不是2个或3个) ".to_string()
    }

    fn room_create(&self, username: &str, room_name: &str) -> String {
        let mut um = self.user_manager.lock().unwrap();
        let current_room = um.get_user_room(username);
        if current_room.is_some() {
            return "tip 参数错误(已经在房间里) ".to_string();
        }

        if um.get_money(username) < 100 {
            return "tip 参数错误(金币不足) ".to_string();
        }
        um.subtract_money(username, 100);

        drop(um);

        let mut rm = self.room_manager.lock().unwrap();
        if !rm.create_room(room_name.to_string(), username.to_string()) {
            let mut um = self.user_manager.lock().unwrap();
            um.add_money(username, 100);
            return "tip 参数错误(房间名已存在) ".to_string();
        }

        let mut um = self.user_manager.lock().unwrap();
        um.set_user_room(username, Some(room_name.to_string()));

        println!("User {} created room {}", username, room_name);
        self.save();
        format!("CreateRoomSucess {} ", room_name)
    }

    fn room_join(&self, username: &str, room_name: &str, index: usize) -> String {
        let mut um = self.user_manager.lock().unwrap();
        if um.get_user_room(username).is_some() {
            return "tip 参数错误(已经在房间里) ".to_string();
        }
        drop(um);

        let mut rm = self.room_manager.lock().unwrap();
        let room = match rm.get_room_mut(room_name) {
            Some(r) => r,
            None => return "tip 参数错误(房间不存在) ".to_string(),
        };

        if room.is_full() {
            return "tip 参数错误(房间已满) ".to_string();
        }

        room.add_guest(username.to_string());

        let mut um = self.user_manager.lock().unwrap();
        um.set_user_room(username, Some(room_name.to_string()));

        println!("User {} joined room {}", username, room_name);

        if let Some(mut socket) = self.sockets.lock().unwrap().get(&index) {
            socket.write_all(format!("game start {} ", room.belongs_to).as_bytes()).ok();
        }
        if let Some(host_idx) = self.user_manager.lock().unwrap().find_user_index_by_name(&room.belongs_to) {
            if let Some(mut socket) = self.sockets.lock().unwrap().get(&host_idx) {
                socket.write_all(format!("game start {} ", username).as_bytes()).ok();
            }
        }

        format!("JoinRoomSucess {} ", room_name)
    }

    fn room_list(&self) -> String {
        let rm = self.room_manager.lock().unwrap();
        let mut response = format!("nowrooms {} ", rm.rooms.len());
        for (name, room) in &rm.rooms {
            let status = if room.guest.is_some() { "2/2" } else { "1/2" };
            response.push_str(&format!("{}┄({})###", name, status));
        }
        response
    }

    fn room_exit(&self, username: &str) -> String {
        let mut um = self.user_manager.lock().unwrap();
        let room_name = match um.get_user_room(username) {
            Some(r) => r,
            None => return "tip 参数错误(房间不存在) ".to_string(),
        };

        um.set_user_room(username, None);
        let host_name = room_name.clone();
        let guest_name = um.find_user_in_room(&room_name, username);
        drop(um);

        let mut rm = self.room_manager.lock().unwrap();

        if let Some(guest) = guest_name {
            if let Some(guest_idx) = self.user_manager.lock().unwrap().find_user_index_by_name(&guest) {
                if let Some(mut socket) = self.sockets.lock().unwrap().get(&guest_idx) {
                    socket.write_all(b"game exit ").ok();
                }
            }
            let mut um = self.user_manager.lock().unwrap();
            um.set_user_room(&guest, None);
        }

        println!("User {} left room {}, room closed", username, room_name);
        rm.remove_room(&room_name);
        self.save();
        "game exit ".to_string()
    }

    fn handle_game(&self, parts: &[&str], index: usize) -> String {
        let um = self.user_manager.lock().unwrap();
        let username = match um.find_user_by_index(index) {
            Some(u) => u,
            None => return "tip 参数错误(未登录) ".to_string(),
        };

        let room_name = match um.get_user_room(&username) {
            Some(r) => r,
            None => return "tip 参数错误(未在房间中) ".to_string(),
        };
        drop(um);

        if parts.len() < 2 {
            return String::new();
        }

        let action = parts[1];

        match action {
            "start" => {
                let mut rm = self.room_manager.lock().unwrap();
                if let Some(room) = rm.get_room_mut(&room_name) {
                    room.start_game();
                    println!("Room {} game started", room_name);
                    self.send_to_room(&room_name, "game start ");
                }
                String::new()
            }
            "nowinfo" => {
                let rm = self.room_manager.lock().unwrap();
                if let Some(room) = rm.get_room(&room_name) {
                    let info = GameLogic::build_room_info(room, &username);
                    if let Some(mut socket) = self.sockets.lock().unwrap().get(&index) {
                        socket.write_all(info.as_bytes()).ok();
                    }
                }
                String::new()
            }
            "pass" => {
                if parts.len() < 3 {
                    return "tip 参数错误 ".to_string();
                }
                let card_index: usize = parts[2].parse().unwrap_or(0);
                self.handle_pass(&username, &room_name, card_index, index)
            }
            "next" => self.handle_next(&username, &room_name, index),
            "use" => {
                if parts.len() < 3 {
                    return "tip 参数错误 ".to_string();
                }
                let card_index: usize = parts[2].parse().unwrap_or(0);
                self.handle_use(&username, &room_name, card_index, index)
            }
            "chat" => self.handle_chat(&username, &room_name, parts, index),
            _ => String::new(),
        }
    }

    fn handle_pass(&self, username: &str, room_name: &str, card_index: usize, index: usize) -> String {
        let mut rm = self.room_manager.lock().unwrap();
        let room = match rm.get_room_mut(room_name) {
            Some(r) => r,
            None => return "tip 参数错误(房间不存在) ".to_string(),
        };

        if !room.is_player_turn(username) {
            return "tip 参数错误(不是该玩家的回合) ".to_string();
        }

        let player_index = if room.is_player1(username) { 1 } else { 2 };
        let result = GameLogic::pass_card(room, player_index, card_index);

        match result {
            crate::game::PassResult::Success => {
                let msg1 = "log 你放置了一张被动卡牌 ";
                let msg2 = "log 对方放置了一张被动卡牌 ";
                self.send_to_room(room_name, msg1);
                self.send_to_room(room_name, msg2);
                self.room_refresh(room_name);
                String::new()
            }
            crate::game::PassResult::Error(e) => format!("tip {} ", e),
        }
    }

    fn handle_next(&self, username: &str, room_name: &str, index: usize) -> String {
        let mut rm = self.room_manager.lock().unwrap();
        let room = match rm.get_room_mut(room_name) {
            Some(r) => r,
            None => return "tip 参数错误(房间不存在) ".to_string(),
        };

        if !room.is_player_turn(username) {
            return "tip 参数错误(不是该玩家的回合) ".to_string();
        }

        GameLogic::next_turn(room);
        drop(rm);
        self.room_refresh(room_name);
        String::new()
    }

    fn handle_use(&self, username: &str, room_name: &str, card_index: usize, _index: usize) -> String {
        let mut rm = self.room_manager.lock().unwrap();
        
        let game_ended;
        {
            let room = match rm.get_room_mut(room_name) {
                Some(r) => r,
                None => return "tip 参数错误(房间不存在) ".to_string(),
            };

            if !room.is_player_turn(username) {
                return "tip 参数错误(不是该玩家的回合) ".to_string();
            }

            let player_index = if room.is_player1(username) { 1 } else { 2 };
            let result = GameLogic::use_card(room, player_index, card_index);

            match result {
                crate::game::UseCardResult::Error(e) => return format!("tip {} ", e),
                crate::game::UseCardResult::Attack { .. } => {
                    GameLogic::next_turn(room);
                    game_ended = GameLogic::check_game_end(room);
                }
                _ => {
                    game_ended = None;
                }
            }
        }
        
        drop(rm);
        self.room_refresh(room_name);
        
        if let Some(result) = game_ended {
            return self.handle_game_result(room_name, result);
        }
        
        String::new()
    }

    fn handle_game_result(&self, room_name: &str, result: GameResult) -> String {
        match result {
            GameResult::Player1Win => {
                self.send_to_room(room_name, "game end win ");
                println!("Room {} player1 won", room_name);
                self.close_room(room_name);
                String::new()
            }
            GameResult::Player2Win => {
                self.send_to_room(room_name, "game end loss ");
                println!("Room {} player2 won", room_name);
                self.close_room(room_name);
                String::new()
            }
            GameResult::Draw => {
                self.send_to_room(room_name, "game end p ");
                println!("Room {} draw", room_name);
                self.close_room(room_name);
                String::new()
            }
            GameResult::Continue => String::new(),
        }
    }

    fn handle_chat(&self, username: &str, room_name: &str, parts: &[&str], _index: usize) -> String {
        if parts.len() < 3 {
            return String::new();
        }

        let message = parts[2..].join(" ");
        println!("User {} in room {} said: {}", username, room_name, message);

        let chat_msg = format!("game log {}:{} ", username, message);
        self.send_to_room(room_name, &chat_msg);
        String::new()
    }

    fn handle_test(&self, parts: &[&str], index: usize) -> String {
        if parts.len() < 2 {
            return String::new();
        }

        if parts[1] == "moneyadd1" {
            let mut um = self.user_manager.lock().unwrap();
            let username = match um.find_user_by_index(index) {
                Some(u) => u,
                None => return "tip 参数错误(未登录) ".to_string(),
            };

            um.add_money(&username, 10);
            drop(um);
            self.save();

            let um = self.user_manager.lock().unwrap();
            let money = um.get_money(&username);
            let online_count = um.online_users.len();
            println!("User {} gained 10 coins", username);
            return format!("selfinfo {} {} {} ", username, money, online_count);
        }

        String::new()
    }

    fn handle_disconnect(&self, index: usize) {
        let (username, room_name) = {
            let um = self.user_manager.lock().unwrap();
            match um.find_user_by_index(index) {
                Some(u) => (Some(u.clone()), um.get_user_room(&u)),
                None => (None, None),
            }
        };

        if let (Some(user), Some(room)) = (username.clone(), room_name.clone()) {
            self.send_to_room(&room, "game exit ");

            let mut rm = self.room_manager.lock().unwrap();
            rm.remove_room(&room);

            let mut um = self.user_manager.lock().unwrap();
            um.set_user_room(&user, None);
            println!("User {} disconnected, room {} closed", user, room);
        }

        if let Some(user) = username {
            let mut um = self.user_manager.lock().unwrap();
            um.remove_online_user(&user);
            println!("User {} disconnected", user);
        }

        self.sockets.lock().unwrap().remove(&index);
    }

    fn room_refresh(&self, room_name: &str) {
        let rm = self.room_manager.lock().unwrap();
        let room = match rm.get_room(room_name) {
            Some(r) => r,
            None => return,
        };

        let player1 = room.belongs_to.clone();
        let player2 = room.guest.clone();

        drop(rm);

        let um = self.user_manager.lock().unwrap();

        if let Some(idx) = um.find_user_index_by_name(&player1) {
            if let Some(mut socket) = self.sockets.lock().unwrap().get(&idx) {
                let rm = self.room_manager.lock().unwrap();
                if let Some(room) = rm.get_room(room_name) {
                    let info = GameLogic::build_room_info(room, &player1);
                    socket.write_all(info.as_bytes()).ok();
                }
            }
        }

        if let Some(ref p2) = player2 {
            if let Some(idx) = um.find_user_index_by_name(p2) {
                if let Some(mut socket) = self.sockets.lock().unwrap().get(&idx) {
                    let rm = self.room_manager.lock().unwrap();
                    if let Some(room) = rm.get_room(room_name) {
                        let info = GameLogic::build_room_info(room, p2);
                        socket.write_all(info.as_bytes()).ok();
                    }
                }
            }
        }
    }

    fn send_to_room(&self, room_name: &str, msg: &str) {
        let (player1, player2_opt) = {
            let rm = self.room_manager.lock().unwrap();
            match rm.get_room(room_name) {
                Some(r) => (r.belongs_to.clone(), r.guest.clone()),
                None => return,
            }
        };

        let um = self.user_manager.lock().unwrap();

        if let Some(idx) = um.find_user_index_by_name(&player1) {
            if let Some(mut socket) = self.sockets.lock().unwrap().get(&idx) {
                socket.write_all(msg.as_bytes()).ok();
            }
        }

        if let Some(player2) = player2_opt {
            if let Some(idx) = um.find_user_index_by_name(&player2) {
                if let Some(mut socket) = self.sockets.lock().unwrap().get(&idx) {
                    socket.write_all(msg.as_bytes()).ok();
                }
            }
        }
    }

    fn close_room(&self, room_name: &str) {
        self.send_to_room(room_name, "game exit ");

        let mut rm = self.room_manager.lock().unwrap();
        if let Some(room) = rm.rooms.remove(room_name) {
            let mut um = self.user_manager.lock().unwrap();
            um.set_user_room(&room.belongs_to, None);
            if let Some(guest) = &room.guest {
                um.set_user_room(guest, None);
            }
            println!("Room {} closed", room_name);
        }
    }
}

impl Clone for Server {
    fn clone(&self) -> Self {
        Self {
            version: self.version.clone(),
            protocol_version: self.protocol_version,
            host: self.host.clone(),
            port: self.port,
            user_manager: Mutex::new(UserManager::new()),
            room_manager: Mutex::new(RoomManager::new()),
            email_service: Mutex::new(None),
            new_index: Mutex::new(0),
            debug_mode: Mutex::new(*self.debug_mode.lock().unwrap()),
            sockets: Mutex::new(HashMap::new()),
        }
    }
}