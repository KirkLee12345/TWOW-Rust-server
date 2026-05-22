use std::net::TcpStream;
use std::sync::{Mutex, OnceLock};
use rand::Rng;
use crate::lib::server::log;

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

pub fn remove_room_by_room_name(room_name: &String, client_index: usize) {
    let mut flag = false;
    for room in get_rooms().lock().unwrap().iter_mut() {
        if room.name == *room_name {
            if room.now != 3 {
                room.now = 3;
                log(format!("[{client_index}] 房间 {room_name} 对局完成，即将关闭"));
                return;
            }
            if room.now == 3 {
                flag = true;
                break;
            }
        }
    }
    if !flag { return; }
    log(format!("[{client_index}] 房间 {room_name} 对局完成，已关闭"));
    let mut rooms = get_rooms().lock().unwrap();
    if let Some(pos) = rooms.iter().position(|r| r.name == *room_name) {
        rooms.remove(pos);
    }
}

pub fn room_refresh(user_name: &String) -> String {
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
                // TODO
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
                // TODO
            }
            r.push(' ');
            return r;
        }
    }
    unreachable!();
}
