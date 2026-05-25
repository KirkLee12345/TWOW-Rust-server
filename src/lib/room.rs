use std::cmp::PartialEq;
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
    Attack(i32),
    Shield(i32),
    AddEnergy(i32),
    ConsumeEnergy(i32),
    Skill(i32),
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
    pub player: [Player; 2],
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
            player: [Player::default(), Player::default()],
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
    pub fn to_string(&self) -> String {
        match self {
            Card::Empty => "空卡牌".to_string(),
            Card::Attack(0) => "攻击反转".to_string(),
            Card::Shield(0) => "护盾反转".to_string(),
            Card::AddEnergy(0) => "能量反转".to_string(),
            Card::ConsumeEnergy(0) => "扣能反转".to_string(),
            Card::Attack(i) => format!("{i}点攻击牌"),
            Card::Shield(i) => format!("{i}点护盾牌"),
            Card::AddEnergy(i) => format!("{i}点能量牌"),
            Card::ConsumeEnergy(i) => format!("{i}点扣能牌"),
            Card::Skill(i) => format!("{i}点回血牌"),
        }
    }
}

impl Room {
    pub fn get_random_card_to_player(&mut self, p: usize, thread_index: usize) {
        if self.all_cards.is_empty() {
            if IS_DEBUG { log(format!("[{thread_index}] 发送数据: game end p ")); }
            get_client_by_user_name(&self.belongs_to).unwrap().write_all("game end p ".as_bytes()).unwrap();
            if IS_DEBUG { log(format!("[{thread_index}] 发送数据: game end p ")); }
            get_client_by_user_name(&self.guest).unwrap().write_all("game end p ".as_bytes()).unwrap();
            log(format!("房间 {} 平局", self.name));
            return;
        }
        let empty_slot = match self.player[p].hand_cards.iter_mut()
            .find(|slot| matches!(slot, Card::Empty))
        {
            Some(slot) => slot,
            None => return,
        };
        let idx = rand::thread_rng().gen_range(0..self.all_cards.len());
        *empty_slot = self.all_cards.swap_remove(idx);
    }
    pub fn remove_random_card_for_player(&mut self, p: usize, thread_index: usize) {
        let mut temp_index: Vec<usize> = vec![];
        for i in 0..self.player[p].hand_cards.len() {
            match self.player[p].hand_cards[i] {
                Card::Empty => continue,
                _ => {
                    temp_index.push(i);
                    break;
                }
            }
        }
        if temp_index.len() == 0 { return; }
        let idx = rand::thread_rng().gen_range(0..temp_index.len());
        self.player[p].hand_cards[temp_index[idx]] = Card::Empty;
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
    pub fn panduan_player_is_can_continue(&mut self, thread_index:usize, p: usize) -> bool {
        let mut cnt: u8 = 0;
        for i in 0..8 {
            match self.player[p].hand_cards[i] {
                Card::Empty => continue,
                _ => cnt += 1,
            }
        }
        if cnt == 0 {
            for i in 0..2 {
                if let Card::Skill(num) = self.player[p].passive_cards[i] {
                    self.player[p].passive_cards[i] = Card::Empty;
                    for _ in 0..num { self.get_random_card_to_player(p, thread_index); }
                    let text1 = format!("log 你的被动卡牌被触发了，你摸了{num}张卡牌继续战斗!");
                    let text2 = format!("log 对方被动卡牌被触发了，对方摸了{num}张卡牌继续战斗!");
                    if p == 0 { self.log(thread_index, &self.belongs_to, text1, text2); }
                    else { self.log(thread_index, &self.guest, text1, text2); }
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
            log(format!("[{thread_index}] 向 {} 房间的房主玩家 {} 发送信息 {r1}", self.name, self.belongs_to));
            log(format!("[{thread_index}] 向 {} 房间的访客玩家 {} 发送信息 {r2}", self.name, self.guest));
        } else {
            self.get_player1_client().unwrap().write_all(r2.as_bytes()).unwrap();
            self.get_player2_client().unwrap().write_all(r1.as_bytes()).unwrap();
            log(format!("[{thread_index}] 向 {} 房间的房主玩家 {} 发送信息 {r2}", self.name, self.belongs_to));
            log(format!("[{thread_index}] 向 {} 房间的访客玩家 {} 发送信息 {r1}", self.name, self.guest));
        }
        thread::sleep(Duration::from_millis(SLPPE_TIME_MILLIS));
    }
    pub fn pass(&mut self, thread_index: usize, user_name: &String, card_index: usize) -> String {
        let mut p: usize = 0;
        if self.belongs_to == *user_name { p = 0; }
        if self.guest == *user_name { p = 1; }
        match self.player[p].hand_cards[card_index] {
            Card::AddEnergy(0) => (),
            Card::Attack(0) => (),
            Card::ConsumeEnergy(0) => (),
            Card::Shield(0) => (),
            Card::Skill(_) => (),
            Card::Empty => return "tip [E129]参数错误(该手牌不存在) ".to_string(),
            _ => return "tip [E130]参数错误(该手牌不能作为被动卡牌) ".to_string(),
        }
        for i in 0..2 {
            match self.player[p].passive_cards[i] {
                Card::Empty => {
                    self.player[p].passive_cards[i] = self.player[p].hand_cards[card_index];
                    self.player[p].hand_cards[card_index] = Card::Empty;
                    self.log(thread_index, user_name, format!("log 你放置了一张被动卡牌{}", self.player[p].hand_cards[card_index].to_string()), format!("log 对方放置了一张被动卡牌{}", self.player[p].hand_cards[card_index].to_string()));
                    return "null".to_string();
                },
                _ => (),
            }
        }
        "tip [E131]参数错误(被动卡槽已满) ".to_string()
    }
    pub fn nnext(&mut self, thread_index: usize) -> bool {
        if self.player[self.now-1].energy < 6 {
            self.player[self.now-1].energy += 2;
            if self.player[self.now-1].energy > 6 {
                self.player[self.now-1].energy = 6;
            }
        }
        if !self.player[self.now-1].used {
            self.get_random_card_to_player(self.now-1, thread_index)
        }
        self.player[self.now-1].used = false;
        if self.now == 1 { self.now = 2}
        else { self.now = 1}
        false
    }
    pub fn use_card(&mut self, thread_index: usize, user_name: &String, card_index: usize) -> String {
        let mut p: usize = 0;
        let mut pp: usize = 1;
        if *user_name == self.belongs_to {
            p = 0;
            pp = 1;
        }
        if *user_name == self.guest {
            p = 1;
            pp = 0;
        }
        match self.player[p].hand_cards[card_index] {
            Card::Empty => return "tip [E128]参数错误(该手牌不存在) ".to_string(),
            Card::Skill(num) => {
                self.log(thread_index, user_name, format!("log 你打出了一张{}，摸了{num}张牌 ", self.player[p].hand_cards[card_index].to_string()), format!("log 对方打出了一张{}，摸了{num}张牌 ", self.player[p].hand_cards[card_index].to_string()));
                self.player[p].hand_cards[card_index] = Card::Empty;
                self.player[p].used = true;
                self.last_card = Card::Skill(num);
                for _ in 0..num {
                    self.get_random_card_to_player(p, thread_index);
                }
                return "null".to_string();
            }
            Card::AddEnergy(num) => {
                self.add_energy(thread_index, p, num, format!("log 你打出了一张{}", self.player[p].hand_cards[card_index].to_string()), format!("log 对方打出了一张{}", self.player[p].hand_cards[card_index].to_string()));
                self.player[p].hand_cards[card_index] = Card::Empty;
                self.last_card = Card::AddEnergy(num);
                self.player[p].used = true;
                return "null".to_string();
            }
            Card::ConsumeEnergy(num) => {
                self.consume_energy(thread_index, p, num, format!("log 你打出了一张{}", self.player[p].hand_cards[card_index].to_string()), format!("log 对方打出了一张{}", self.player[p].hand_cards[card_index].to_string()));
                self.player[p].hand_cards[card_index] = Card::Empty;
                self.last_card = Card::ConsumeEnergy(num);
                self.player[p].used = true;
                return "null".to_string();
            }
            Card::Shield(num) => {
                if self.player[p].energy < num { return "tip [E132]参数错误(能量不足) ".to_string(); }
                self.player[p].energy -= num;
                if !self.defend(thread_index, p, num, format!("log 你打出了一张{}", self.player[p].hand_cards[card_index].to_string()), format!("log 对方打出了一张{}", self.player[p].hand_cards[card_index].to_string())) {
                    self.player[p].energy += num;
                    return "tip [E133]参数错误(盾牌槽已满) ".to_string();
                }
                self.last_card = Card::Shield(num);
                self.player[p].hand_cards[card_index] = Card::Empty;
                self.player[p].used = true;
                return "null".to_string();
            }
            Card::Attack(num) => {
                if self.player[p].energy < num { return "tip [E134]参数错误(能量不足) ".to_string(); }
                self.player[p].hand_cards[card_index] = Card::Empty;
                self.last_card = Card::Attack(num);
                self.player[p].energy -= num;
                self.player[p].used = true;
                self.damage(thread_index, p, num, format!("log 你打出了一张{}", self.player[p].hand_cards[card_index].to_string()), format!("log 对方打出了一张{}", self.player[p].hand_cards[card_index].to_string()));
                self.nnext(thread_index);
                return "null".to_string();
            }
        }
        unreachable!()
    }
    pub fn add_energy(&mut self, thread_index: usize, p: usize, num: i32, mut text1: String, mut text2: String) {
        let pp: usize = if p == 0 { 1 } else { 0 };
        for i in 0..self.player[pp].passive_cards.len() {
            match self.player[pp].passive_cards[i] {
                Card::AddEnergy(0) => {
                    self.player[pp].passive_cards[i] = Card::Empty;
                    text1.push_str(format!("，但触发了对方的被动卡牌{}", Card::AddEnergy(0).to_string()).as_str());
                    text2.push_str(format!("，但触发了你的被动卡牌{}", Card::AddEnergy(0).to_string()).as_str());
                    self.add_energy(thread_index, pp, num, text2, text1);
                    return;
                },
                _ => (),
            }
        }
        self.player[p].energy += num;
        text1.push_str("。 ");
        text2.push_str("。 ");
        if p == 0 { self.log(thread_index, &self.belongs_to, text1, text2); }
        else { self.log(thread_index, &self.guest, text1, text2); }
    }
    pub fn consume_energy(&mut self, thread_index: usize, p: usize, num: i32, mut text1: String, mut text2: String) {
        let pp: usize = if p == 0 { 1 } else { 0 };
        for i in 0..self.player[pp].passive_cards.len() {
            match self.player[pp].passive_cards[i] {
                Card::ConsumeEnergy(0) => {
                    self.player[pp].passive_cards[i] = Card::Empty;
                    text1.push_str(format!("，但触发了对方的被动卡牌{}", Card::ConsumeEnergy(0).to_string()).as_str());
                    text2.push_str(format!("，但触发了你的被动卡牌{}", Card::ConsumeEnergy(0).to_string()).as_str());
                    self.consume_energy(thread_index, pp, num, text2, text1);
                    return;
                },
                _ => (),
            }
        }
        self.player[pp].energy -= num;
        text1.push_str("。 ");
        text2.push_str("。 ");
        if p == 0 { self.log(thread_index, &self.belongs_to, text1, text2); }
        else { self.log(thread_index, &self.guest, text1, text2); }
    }
    pub fn defend(&mut self, thread_index: usize, p: usize, num: i32, mut text1: String, mut text2: String) -> bool {
        let pp: usize = if p == 0 { 1 } else { 0 };
        for i in 0..self.player[pp].passive_cards.len() {
            match self.player[pp].passive_cards[i] {
                Card::Shield(0) => {
                    self.player[pp].passive_cards[i] = Card::Empty;
                    text1.push_str(format!("，但触发了对方的被动卡牌{}", Card::Shield(0).to_string()).as_str());
                    text2.push_str(format!("，但触发了你的被动卡牌{}", Card::Shield(0).to_string()).as_str());
                    if !self.defend(thread_index, pp, num, text2.clone(), text1.clone()) {
                        text1.push_str("，但对方的护盾槽已满，自动反转回来");
                        text2.push_str("，但你的护盾槽已满，自动反转回去");
                        return self.defend(thread_index, p, num, text1, text2);
                    }
                    return true;
                },
                _ => (),
            }
        }
        for i in 0..self.player[p].out_cards.len() {
            match self.player[p].out_cards[i] {
                Card::Empty => {
                    self.player[p].out_cards[i] = Card::Shield(num);
                    text1.push_str("。 ");
                    text2.push_str("。 ");
                    if p == 0 { self.log(thread_index, &self.belongs_to, text1, text2); }
                    else { self.log(thread_index, &self.guest, text1, text2); }
                    return true;
                },
                _ => (),
            }
        }
        false
    }
    pub fn damage(&mut self, thread_index: usize, p: usize, mut num: i32, mut text1: String, mut text2: String) {
        let pp: usize = if p == 0 { 1 } else { 0 };
        for i in 0..self.player[pp].passive_cards.len() {
            match self.player[pp].passive_cards[i] {
                Card::Attack(0) => {
                    self.player[pp].passive_cards[i] = Card::Empty;
                    text1.push_str(format!("，但触发了对方的被动卡牌{}", Card::Attack(0).to_string()).as_str());
                    text2.push_str(format!("，但触发了你的被动卡牌{}", Card::Attack(0).to_string()).as_str());
                    self.damage(thread_index, pp, num, text2, text1);
                    return;
                },
                _ => (),
            }
        }
        for i in 0..self.player[pp].out_cards.len() {
            match self.player[pp].out_cards[i] {
                Card::Shield(1) => {
                    self.player[pp].out_cards[i] = Card::Empty;
                    text1.push_str(format!("，被对方的1点无敌护盾卡牌抵消了所有{num}点伤害").as_str());
                    text2.push_str(format!("，被你的1点无敌护盾卡牌抵消了所有{num}点伤害").as_str());
                    num = 0;
                }
                Card::Shield(snum) => {
                    if num == snum {
                        self.player[pp].out_cards[i] = Card::Empty;
                        text1.push_str(format!("，被对方的{snum}点护盾卡牌刚好抵消了所有{num}点伤害").as_str());
                        text2.push_str(format!("，被你的{snum}点护盾卡牌刚好抵消了所有{num}点伤害").as_str());
                        num = 0;
                    } else if num < snum {
                        self.player[pp].out_cards[i] = Card::Shield(snum - num);
                        text1.push_str(format!("，被对方的{snum}点护盾卡牌完全抵消了{num}点伤害").as_str());
                        text2.push_str(format!("，被你的{snum}点护盾卡牌完全抵消了{num}点伤害").as_str());
                        num = 0;
                    } else {
                        self.player[pp].out_cards[i] = Card::Empty;
                        text1.push_str(format!("，被对方的{snum}点护盾卡牌抵消了部分伤害").as_str());
                        text2.push_str(format!("，被你的{snum}点护盾卡牌抵消了部分伤害").as_str());
                        num -= snum;
                    }
                }
                _ => (),
            }
            if num == 0 { break; }
        }
        if num != 0 {
            for _ in 0..num {
                self.remove_random_card_for_player(pp, thread_index);
            }
            text1.push_str(format!("，对对方造成{num}点伤害").as_str());
            text2.push_str(format!("，对你造成{num}点伤害").as_str());
        }
        text1.push_str("。 ");
        text2.push_str("。 ");
        if p == 0 { self.log(thread_index, &self.belongs_to, text1, text2); }
        else { self.log(thread_index, &self.guest, text1, text2); }
    }
}

pub fn room_start(room_name: &String, thread_index: usize) {
    for room in get_rooms().lock().unwrap().iter_mut() {
        if room.name == *room_name {
            if room.now != 0 { return; }
            room.now = 1;
            room.player[0].energy = 4;
            room.player[1].energy = 4;
            room.init_all_cards();
            for _ in 0..6 {
                room.get_random_card_to_player(0, thread_index);
                room.get_random_card_to_player(1, thread_index);
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
    let mut is_need_clear_room = false;
    for room in get_rooms().lock().unwrap().iter_mut() {
        if room.name == room_name {
            let mut r = "game nowinfo".to_string();
            let mut p = 0;
            let mut pp = 1;
            if room.belongs_to == *user_name {
                p = 0;
                pp = 1;
            } else {
                p = 1;
                pp = 0;
            }
            for i in room.player[p].hand_cards {
                r.push(' ');
                r.push_str(i.to_str().as_str())
            }
            for i in room.player[p].passive_cards {
                r.push(' ');
                r.push_str(i.to_str().as_str())
            }
            for i in room.player[pp].hand_cards {
                r.push(' ');
                match i {
                    Card::Empty => r.push('0'),
                    _ => r.push('b')
                }
            }
            for i in room.player[pp].passive_cards {
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
            for i in room.player[p].out_cards {
                r.push(' ');
                r.push_str(i.to_str().as_str())
            }
            for i in room.player[pp].out_cards {
                r.push(' ');
                r.push_str(i.to_str().as_str())
            }
            r.push(' ');
            r.push_str(room.player[p].energy.to_string().as_str());
            r.push(' ');
            r.push_str(room.player[pp].energy.to_string().as_str());
            r.push(' ');
            r.push_str(room.all_cards.len().to_string().as_str());
            r.push(' ');
            if room.now == 1 {
                if p == 0 { r.push('1'); }
                else { r.push('0'); }
            } else {
                if pp == 0 { r.push('1'); }
                else { r.push('0'); }
            }
            r.push(' ');
            r.push_str(room.last_card.to_str().as_str());
            if !room.panduan_player_is_can_continue(thread_index, p) {
                if IS_DEBUG {log(format!("[{thread_index}] 发送数据: game end loss "));}
                get_client_by_user_name(user_name).unwrap().write_all("game end loss ".as_bytes()).unwrap();
                log(format!("[{thread_index}] 房间 {room_name} 玩家 {user_name} 输了"));
                is_need_clear_room = true;
                thread::sleep(Duration::from_millis(SLPPE_TIME_MILLIS));
            }
            if !room.panduan_player_is_can_continue(thread_index, pp) {
                if IS_DEBUG {log(format!("[{thread_index}] 发送数据: game end win "));}
                get_client_by_user_name(user_name).unwrap().write_all("game end win ".as_bytes()).unwrap();
                log(format!("[{thread_index}] 房间 {room_name} 玩家 {user_name} 赢了"));
                is_need_clear_room = true;
                thread::sleep(Duration::from_millis(SLPPE_TIME_MILLIS));
            }
            r.push(' ');
            if IS_DEBUG {log(format!("[{thread_index}] 发送数据: {r}"));}
            get_client_by_user_name(user_name).unwrap().write_all(r.as_bytes()).unwrap();
            if !is_need_clear_room { return; };
        }
    }
    remove_room_by_room_name(&room_name, thread_index);
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

pub(crate) fn room_next(thread_index: usize, user_name: &String) {
    let mut player1_name = "".to_string();
    let mut player2_name = "".to_string();
    let mut is_game_p = false;
    let room_name = get_room_name_by_user(user_name);
    for room in get_rooms().lock().unwrap().iter_mut() {
        if room.name == room_name {
            is_game_p = room.nnext(thread_index);
            player1_name = room.belongs_to.clone();
            player2_name = room.guest.clone();
            break;
        }
    }
    room_refresh(thread_index, &player1_name);
    room_refresh(thread_index, &player2_name);
    if is_game_p {
        thread::sleep(Duration::from_millis(SLPPE_TIME_MILLIS));
        remove_room_by_room_name(&room_name, thread_index);
        remove_room_by_room_name(&room_name, thread_index);
    }
}

pub(crate) fn room_use(thread_index: usize, user_name: &mut String, card_index: usize) -> String {
    let mut player1_name = "".to_string();
    let mut player2_name = "".to_string();
    let room_name = get_room_name_by_user(user_name);
    let mut r = "null".to_string();
    if let Some(room) = get_rooms().lock().unwrap().iter_mut().find(|r| r.name == room_name) {
        player1_name = room.belongs_to.clone();
        player2_name = room.guest.clone();
        r = room.use_card(thread_index, user_name, card_index);
    }
    room_refresh(thread_index, &player1_name);
    room_refresh(thread_index, &player2_name);
    thread::sleep(Duration::from_millis(SLPPE_TIME_MILLIS));
    r
}
