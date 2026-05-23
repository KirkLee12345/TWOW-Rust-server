use std::io::Write;
use std::net::TcpStream;
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;
use rand::Rng;
use crate::lib::game::get_client_by_user_name;
use crate::lib::server::{log, IS_DEBUG, SLPPE_TIME_MILLIS};


pub(crate) static ROOMS: OnceLock<Mutex<Vec<Room>>> = OnceLock::new();

#[derive(Clone, Copy)]
pub enum Card {
    Empty,
    Attack(i8),
    Shield(i8),
    AddEnergy(i8),
    ConsumeEnergy(i8),
    Skill(i8),
}

pub struct Player {
    pub used: bool,
    pub energy: i32,
    pub hand_cards: [Card; 8],
    pub passive_cards: [Card; 2],
    pub out_cards: [Card; 3],
}

pub struct Room {
    pub name: String,
    pub belongs_to: String,
    pub guest: String,
    pub now: usize,
    pub player1: Player,
    pub player2: Player,
    pub last_card: Card,
    pub all_cards: Vec<Card>,
}

impl Default for Card {
    fn default() -> Self {
        Card::Empty
    }
}

impl Default for Player {
    fn default() -> Self {
        Player {
            used: false,
            energy: 0,
            hand_cards: [Card::Empty; 8],
            passive_cards: [Card::Empty; 2],
            out_cards: [Card::Empty; 3],
        }
    }
}

impl Default for Room {
    fn default() -> Self {
        Room {
            name: "".to_string(),
            belongs_to: "".to_string(),
            guest: "".to_string(),
            now: 0,
            player1: Player::default(),
            player2: Player::default(),
            last_card: Card::Empty,
            all_cards: vec![],
        }
    }
}


pub(crate) fn get_rooms() -> &'static Mutex<Vec<Room>> {
    ROOMS.get_or_init(|| Mutex::new(vec![]))
}

pub fn get_room_name_by_user(name: &String) -> String {
    for r in get_rooms().lock().unwrap().iter() {
        if r.belongs_to == *name || r.guest == *name {
            return r.name.clone();
        }
    }
    "".to_string()
}

pub fn get_other_client_by_user(name: &String) -> Option<TcpStream> {
    let mut other_name = "".to_string();
    for r in get_rooms().lock().unwrap().iter() {
        if r.belongs_to == *name {
            other_name = r.guest.clone();
            break;
        }
        if r.guest == *name {
            other_name = r.belongs_to.clone();
            break;
        }
    }
    if other_name == "" {
        return None;
    }
    for (u, c) in crate::lib::game::get_online_users().lock().unwrap().iter() {
        if *u == other_name {
            return Some(c.try_clone().unwrap());
        }
    }
    None
}

pub fn check_is_in_room_by_user(name: &String) -> bool {
    for r in get_rooms().lock().unwrap().iter() {
        if r.belongs_to == *name || r.guest == *name {
            return true;
        }
    }
    false
}

impl Card {
    pub fn to_str(&self) -> String {
        match self {
            Card::Empty => "0".to_string(),
            Card::Attack(i) => "g".to_string() + &i.to_string(),
            Card::Shield(i) => "d".to_string() + &i.to_string(),
            Card::AddEnergy(i) => "n".to_string() + &i.to_string(),
            Card::ConsumeEnergy(i) => "k".to_string() + &i.to_string(),
            Card::Skill(i) => "w".to_string() + &i.to_string(),
        }
    }
}

impl Room {
    pub fn get_random_card_to_player1(&mut self) -> bool {
        if self.all_cards.is_empty() {
            return false;
        }
        let empty_slot = match self.player1.hand_cards.iter_mut()
            .find(|slot| matches!(slot, Card::Empty))
        {
            Some(slot) => slot,
            None => return true,
        };
        let idx = rand::thread_rng().gen_range(0..self.all_cards.len());
        *empty_slot = self.all_cards.swap_remove(idx);
        true
    }
    pub fn get_random_card_to_player2(&mut self) -> bool {
        if self.all_cards.is_empty() {
            return false;
        }
        let empty_slot = match self.player2.hand_cards.iter_mut()
            .find(|slot| matches!(slot, Card::Empty))
        {
            Some(slot) => slot,
            None => return true,
        };
        let idx = rand::thread_rng().gen_range(0..self.all_cards.len());
        *empty_slot = self.all_cards.swap_remove(idx);
        true
    }
    pub fn init_all_cards(&mut self) {
        for _ in 0..4 {
            for i in 0..=9 {
                self.all_cards.push(Card::Attack(i));
                self.all_cards.push(Card::Shield(i));
                self.all_cards.push(Card::AddEnergy(i));
                self.all_cards.push(Card::ConsumeEnergy(i));
            }
            self.all_cards.push(Card::Skill(2));
            self.all_cards.push(Card::Skill(4));
        }
    }
    pub fn panduan_player1_is_can_continue(&mut self, thread_index:usize) -> bool {
        let mut cnt: u8 = 0;
        for i in 0..8 {
            match self.player1.hand_cards[i] {
                Card::Empty => continue,
                _ => cnt += 1,
            }
        }
        if cnt == 0 {
            for i in 0..2 {
                if let Card::Skill(num) = self.player1.passive_cards[i] {
                    self.player1.passive_cards[i] = Card::Empty;
                    for _ in 0..num { self.get_random_card_to_player1(); }
                    let text1 = format!("log 你的被动卡牌被触发了，你摸了{num}张卡牌继续战斗!");
                    let text2 = format!("log 对方被动卡牌被触发了，对方摸了{num}张卡牌继续战斗!");
                    self.log(thread_index, &self.belongs_to, text1, text2);
                    return true;
                }
            }
            false
        } else {
            true
        }
    }
    pub fn panduan_player2_is_can_continue(&mut self, thread_index:usize) -> bool {
        let mut cnt: u8 = 0;
        for i in 0..8 {
            match self.player2.hand_cards[i] {
                Card::Empty => continue,
                _ => cnt += 1,
            }
        }
        if cnt == 0 {
            for i in 0..2 {
                if let Card::Skill(num) = self.player2.passive_cards[i] {
                    self.player2.passive_cards[i] = Card::Empty;
                    for _ in 0..num { self.get_random_card_to_player2(); }
                    let text1 = format!("log 你的被动卡牌被触发了，你摸了{num}张卡牌继续战斗!");
                    let text2 = format!("log 对方被动卡牌被触发了，对方摸了{num}张卡牌继续战斗!");
                    self.log(thread_index, &self.guest, text1, text2);
                    return true;
                }
            }
            false
        } else {
            true
        }
    }
    pub fn is_belongs_to_user(&self, name: &String) -> bool {
        self.belongs_to == *name
    }
    pub fn get_player1_client(&self) -> Option<TcpStream> {
        for (u, c) in crate::lib::game::get_online_users().lock().unwrap().iter() {
            if *u == self.belongs_to {
                return Some(c.try_clone().unwrap());
            }
        }
        None
    }
    pub fn get_player2_client(&self) -> Option<TcpStream> {
        for (u, c) in crate::lib::game::get_online_users().lock().unwrap().iter() {
            if *u == self.guest {
                return Some(c.try_clone().unwrap());
            }
        }
        None
    }
    pub fn log(&self, thread_index: usize, user_name: &String, text1: String, text2: String) {
        let mut r1: String = "game ".to_string();
        r1.push_str(text1.as_str());
        let mut r2 = "game ".to_string();
        r2.push_str(text2.as_str());
        if self.is_belongs_to_user(user_name) {
            self.get_player1_client().unwrap().write_all(r1.as_bytes()).unwrap();
            self.get_player2_client().unwrap().write_all(r2.as_bytes()).unwrap();
            log(format!("[{thread_index}] 向 {} 房间的房主玩家发送信息 {r1}", self.name));
            log(format!("[{thread_index}] 向 {} 房间的访客玩家发送信息 {r2}", self.name));
        } else {
            self.get_player1_client().unwrap().write_all(r2.as_bytes()).unwrap();
            self.get_player2_client().unwrap().write_all(r1.as_bytes()).unwrap();
            log(format!("[{thread_index}] 向 {} 房间的房主玩家发送信息 {r2}", self.name));
            log(format!("[{thread_index}] 向 {} 房间的访客玩家发送信息 {r1}", self.name));
        }
        thread::sleep(Duration::from_millis(SLPPE_TIME_MILLIS));
    }
    pub fn pass(&mut self, thread_index: usize, user_name: &String, card_index: usize) -> String {
        if self.belongs_to == *user_name {
            match self.player1.hand_cards[card_index] {
                Card::AddEnergy(0) => (),
                Card::Attack(0) => (),
                Card::ConsumeEnergy(0) => (),
                Card::Shield(0) => (),
                Card::Empty => return "tip [E126]参数错误(该手牌不存在) ".to_string(),
                _ => return "tip [E127]参数错误(该手牌不能作为被动卡牌) ".to_string(),
            }
            for i in 0..2 {
                match self.player1.passive_cards[i] {
                    Card::Empty => {
                        self.player1.passive_cards[i] = self.player1.hand_cards[card_index];
                        self.player1.hand_cards[card_index] = Card::Empty;
                        self.log(thread_index, user_name, "log 你放置了一张被动卡牌 ".to_string(), "log 对方放置了一张被动卡牌 ".to_string());
                        return "null".to_string();
                    },
                    _ => (),
                }
            }
            return "tip [E128]参数错误(被动卡槽已满) ".to_string();
        }
        if self.guest == *user_name {
            match self.player2.hand_cards[card_index] {
                Card::AddEnergy(0) => (),
                Card::Attack(0) => (),
                Card::ConsumeEnergy(0) => (),
                Card::Shield(0) => (),
                Card::Empty => return "tip [E129]参数错误(该手牌不存在) ".to_string(),
                _ => return "tip [E130]参数错误(该手牌不能作为被动卡牌) ".to_string(),
            }
            for i in 0..2 {
                match self.player2.passive_cards[i] {
                    Card::Empty => {
                        self.player2.passive_cards[i] = self.player2.hand_cards[card_index];
                        self.player2.hand_cards[card_index] = Card::Empty;
                        self.log(thread_index, user_name, "log 你放置了一张被动卡牌 ".to_string(), "log 对方放置了一张被动卡牌 ".to_string());
                        return "null".to_string();
                    },
                    _ => (),
                }
            }
            return "tip [E131]参数错误(被动卡槽已满) ".to_string();
        }
        unreachable!();
    }
    pub fn nnext(&mut self) {
        if self.now == 1 {
            self.now = 2;
            if self.player1.energy < 6 {
                self.player1.energy += 2;
                if self.player1.energy > 6 {
                    self.player1.energy = 6;
                }
            }
            if !self.player1.used {
                self.get_random_card_to_player1();
            }
            self.player1.used = false;
            return
        }
        if self.now == 2 {
            self.now = 1;
            if self.player2.energy < 6 {
                self.player2.energy += 2;
                if self.player2.energy > 6 {
                    self.player2.energy = 6;
                }
            }
            if !self.player2.used {
                self.get_random_card_to_player2();
            }
            self.player2.used = false;
            return
        }
    }
}

pub fn room_start(room_name: &String) {
    for room in get_rooms().lock().unwrap().iter_mut() {
        if room.name == *room_name {
            if room.now != 0 { return; }
            room.now = 1;
            room.player1.energy = 4;
            room.player2.energy = 4;
            room.init_all_cards();
            for _ in 0..6 {
                room.get_random_card_to_player1();
                room.get_random_card_to_player2();
            }
            break;
        }
    }
}

pub fn remove_room_by_room_name(room_name: &String, thread_index: usize) {
    let mut flag = false;
    for room in get_rooms().lock().unwrap().iter_mut() {
        if room.name == *room_name {
            if room.now != 3 {
                room.now = 3;
                log(format!("[{thread_index}] 房间 {room_name} 对局完成，即将关闭"));
                return;
            }
            if room.now == 3 {
                flag = true;
                break;
            }
        }
    }
    if !flag { return; }
    log(format!("[{thread_index}] 房间 {room_name} 对局完成，已关闭"));
    let mut rooms = get_rooms().lock().unwrap();
    if let Some(pos) = rooms.iter().position(|r| r.name == *room_name) {
        rooms.remove(pos);
    }
}

pub fn is_user_now_in_room(user_name: &String) -> bool {
    for room in get_rooms().lock().unwrap().iter() {
        if room.belongs_to == *user_name && room.now == 1 {
            return true;
        }
        if room.guest == *user_name && room.now == 2 {
            return true;
        }
    }
    false
}

pub fn room_refresh(thread_index: usize, user_name: &String) {
    let room_name = get_room_name_by_user(user_name);
    for room in get_rooms().lock().unwrap().iter_mut() {
        if room.name == room_name {
            let mut r = "game nowinfo".to_string();
            if room.belongs_to == *user_name {
                for i in room.player1.hand_cards {
                    r.push(' ');
                    r.push_str(i.to_str().as_str())
                }
                for i in room.player1.passive_cards {
                    r.push(' ');
                    r.push_str(i.to_str().as_str())
                }
                for i in room.player2.hand_cards {
                    r.push(' ');
                    match i {
                        Card::Empty => r.push('0'),
                        _ => r.push('b')
                    }
                }
                for i in room.player2.passive_cards {
                    r.push(' ');
                    match i {
                        Card::Empty => r.push('0'),
                        _ => r.push('b')
                    }
                }
                r.push(' ');
                if room.all_cards.len() > 0 {
                    r.push('b');
                } else {
                    r.push('0');
                }
                for i in room.player1.out_cards {
                    r.push(' ');
                    r.push_str(i.to_str().as_str())
                }
                for i in room.player2.out_cards {
                    r.push(' ');
                    r.push_str(i.to_str().as_str())
                }
                r.push(' ');
                r.push_str(room.player1.energy.to_string().as_str());
                r.push(' ');
                r.push_str(room.player2.energy.to_string().as_str());
                r.push(' ');
                r.push_str(room.all_cards.len().to_string().as_str());
                r.push(' ');
                if room.now == 1 {
                    r.push('1');
                } else {
                    r.push('0');
                }
                r.push(' ');
                r.push_str(room.last_card.to_str().as_str());
                if !room.panduan_player1_is_can_continue(thread_index) {
                    if IS_DEBUG {log(format!("[{thread_index}] 发送数据: game end loss "));}
                    get_client_by_user_name(user_name).unwrap().write_all("game end loss ".as_bytes()).unwrap();
                    log(format!("[{thread_index}] 房间 {room_name} 玩家 {user_name} 输了"));
                    remove_room_by_room_name(&room_name, thread_index);
                    thread::sleep(Duration::from_millis(SLPPE_TIME_MILLIS));
                }
                if !room.panduan_player2_is_can_continue(thread_index) {
                    if IS_DEBUG {log(format!("[{thread_index}] 发送数据: game end win "));}
                    get_client_by_user_name(user_name).unwrap().write_all("game end win ".as_bytes()).unwrap();
                    log(format!("[{thread_index}] 房间 {room_name} 玩家 {user_name} 赢了"));
                    remove_room_by_room_name(&room_name, thread_index);
                    thread::sleep(Duration::from_millis(SLPPE_TIME_MILLIS));
                }
            } else {
                for i in room.player2.hand_cards {
                    r.push(' ');
                    r.push_str(i.to_str().as_str())
                }
                for i in room.player2.passive_cards {
                    r.push(' ');
                    r.push_str(i.to_str().as_str())
                }
                for i in room.player1.hand_cards {
                    r.push(' ');
                    match i {
                        Card::Empty => r.push('0'),
                        _ => r.push('b')
                    }
                }
                for i in room.player1.passive_cards {
                    r.push(' ');
                    match i {
                        Card::Empty => r.push('0'),
                        _ => r.push('b')
                    }
                }
                r.push(' ');
                if room.all_cards.len() > 0 {
                    r.push('b');
                } else {
                    r.push('0');
                }
                for i in room.player2.out_cards {
                    r.push(' ');
                    r.push_str(i.to_str().as_str())
                }
                for i in room.player1.out_cards {
                    r.push(' ');
                    r.push_str(i.to_str().as_str())
                }
                r.push(' ');
                r.push_str(room.player2.energy.to_string().as_str());
                r.push(' ');
                r.push_str(room.player1.energy.to_string().as_str());
                r.push(' ');
                r.push_str(room.all_cards.len().to_string().as_str());
                r.push(' ');
                if room.now == 2 {
                    r.push('1');
                } else {
                    r.push('0');
                }
                r.push(' ');
                r.push_str(room.last_card.to_str().as_str());
                if !room.panduan_player2_is_can_continue(thread_index) {
                    if IS_DEBUG {log(format!("[{thread_index}] 发送数据: game end loss "));}
                    get_client_by_user_name(user_name).unwrap().write_all("game end loss ".as_bytes()).unwrap();
                    log(format!("[{thread_index}] 房间 {room_name} 玩家 {user_name} 输了"));
                    remove_room_by_room_name(&room_name, thread_index);
                    thread::sleep(Duration::from_millis(SLPPE_TIME_MILLIS));
                }
                if !room.panduan_player1_is_can_continue(thread_index) {
                    if IS_DEBUG {log(format!("[{thread_index}] 发送数据: game end win "));}
                    get_client_by_user_name(user_name).unwrap().write_all("game end win ".as_bytes()).unwrap();
                    log(format!("[{thread_index}] 房间 {room_name} 玩家 {user_name} 赢了"));
                    remove_room_by_room_name(&room_name, thread_index);
                    thread::sleep(Duration::from_millis(SLPPE_TIME_MILLIS));
                }
            }
            r.push(' ');
            if IS_DEBUG {log(format!("[{thread_index}] 发送数据: {r}"));}
            get_client_by_user_name(user_name).unwrap().write_all(r.as_bytes()).unwrap();
            return;
        }
    }
    unreachable!();
}

pub fn room_pass(thread_index: usize, user_name: &mut String, card_index: usize) -> String {
    let mut r = "null".to_string();
    let mut player1_name = "".to_string();
    let mut player2_name = "".to_string();
    let room_name = get_room_name_by_user(user_name);
    for room in get_rooms().lock().unwrap().iter_mut() {
        if room.name == room_name {
            r = room.pass(thread_index, user_name, card_index);
            player1_name = room.belongs_to.clone();
            player2_name = room.guest.clone();

        }
    }
    room_refresh(thread_index, &player1_name);
    room_refresh(thread_index, &player2_name);
    thread::sleep(Duration::from_millis(SLPPE_TIME_MILLIS));
    r
}

pub(crate) fn room_next(thread_index: usize, user_name: &mut String) {
    let mut player1_name = "".to_string();
    let mut player2_name = "".to_string();
    let room_name = get_room_name_by_user(user_name);
    for room in get_rooms().lock().unwrap().iter_mut() {
        if room.name == room_name {
            room.nnext();
            player1_name = room.belongs_to.clone();
            player2_name = room.guest.clone();
            break;
        }
    }
    room_refresh(thread_index, &player1_name);
    room_refresh(thread_index, &player2_name);
}

pub(crate) fn room_use(thread_index: usize, user_name: &mut String, card_index: usize) -> String {
    todo!()
}
