use serde::{Deserialize, Serialize};
use std::time::SystemTime;

fn simple_random(max: usize) -> usize {
    let seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(42);
    ((seed as usize).wrapping_mul(1103515245).wrapping_add(12345)) % max
}

fn give_card_to_player_helper(player: &mut Player, card_opt: Option<String>) -> bool {
    let Some(card) = card_opt else { return false; };
    for slot in player.hand_cards.iter_mut() {
        if *slot == "0" {
            *slot = card;
            return true;
        }
    }
    false
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum RoomState {
    Waiting = 0,
    Player1Turn = 1,
    Player2Turn = 2,
    Finished = 3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub used: bool,
    pub energy: i32,
    pub hand_cards: Vec<String>,
    pub passive_cards: Vec<String>,
    pub out_cards: Vec<String>,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            used: false,
            energy: 0,
            hand_cards: vec!["0".to_string(); 8],
            passive_cards: vec!["0".to_string(); 2],
            out_cards: vec!["0".to_string(); 3],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub belongs_to: String,
    pub guest: Option<String>,
    pub now: RoomState,
    pub player1: Player,
    pub player2: Player,
    pub last_card: String,
    pub all_cards: Vec<String>,
}

impl Room {
    pub fn new(creator: String) -> Self {
        Self {
            belongs_to: creator,
            guest: None,
            now: RoomState::Waiting,
            player1: Player::default(),
            player2: Player::default(),
            last_card: "0".to_string(),
            all_cards: Vec::new(),
        }
    }

    pub fn is_full(&self) -> bool {
        self.guest.is_some()
    }

    pub fn add_guest(&mut self, guest: String) -> bool {
        if self.guest.is_some() {
            false
        } else {
            self.guest = Some(guest);
            true
        }
    }

    pub fn start_game(&mut self) {
        self.now = RoomState::Player1Turn;
        self.player1.energy = 4;
        self.player2.energy = 4;
        
        self.all_cards.clear();
        
        for &s in &["d", "g", "k", "n"] {
            for i in 0..=9 {
                for _ in 0..4 {
                    self.all_cards.push(format!("{}{}", s, i));
                }
            }
        }
        
        for &i in &["2", "4"] {
            for _ in 0..4 {
                self.all_cards.push(format!("w{}", i));
            }
        }
        
        for _ in 0..6 {
            let card1 = self.draw_random_card();
            let card2 = self.draw_random_card();
            let _ = card1.clone();
            let _ = card2.clone();
            give_card_to_player_helper(&mut self.player1, card1);
            give_card_to_player_helper(&mut self.player2, card2);
        }
    }

    pub fn draw_random_card(&mut self) -> Option<String> {
        if self.all_cards.is_empty() {
            return None;
        }
        let idx = simple_random(self.all_cards.len());
        Some(self.all_cards.remove(idx))
    }

    pub fn give_card_to_player(&mut self, player: &mut Player, card_opt: Option<String>) -> bool {
        let Some(card) = card_opt else { return false; };
        
        for slot in player.hand_cards.iter_mut() {
            if *slot == "0" {
                *slot = card;
                return true;
            }
        }
        
        self.all_cards.push(card);
        false
    }

    pub fn random_card_to(&mut self, player: &mut Player) -> bool {
        if self.all_cards.is_empty() {
            return false;
        }
        
        let idx = simple_random(self.all_cards.len());
        let card = self.all_cards.remove(idx);
        
        for slot in player.hand_cards.iter_mut() {
            if *slot == "0" {
                *slot = card;
                return true;
            }
        }
        
        self.all_cards.push(card);
        false
    }

    pub fn remove_random_card(&mut self, player: &mut Player) -> Option<String> {
        let available: Vec<(usize, String)> = player.hand_cards.iter()
            .enumerate()
            .filter(|(_, c)| *c != "0")
            .map(|(i, c)| (i, c.clone()))
            .collect();
        
        if available.is_empty() {
            return None;
        }
        
        let idx = simple_random(available.len());
        let pos = available[idx].0;
        let card = available[idx].1.clone();
        player.hand_cards[pos] = "0".to_string();
        Some(card)
    }

    pub fn get_player_mut(&mut self, is_player1: bool) -> &mut Player {
        if is_player1 { &mut self.player1 } else { &mut self.player2 }
    }

    pub fn is_player1(&self, username: &str) -> bool {
        self.belongs_to == username
    }

    pub fn get_opponent(&self, username: &str) -> Option<String> {
        if self.belongs_to == username {
            self.guest.clone()
        } else {
            Some(self.belongs_to.clone())
        }
    }

    pub fn is_player_turn(&self, username: &str) -> bool {
        let is_p1 = self.is_player1(username);
        match self.now {
            RoomState::Player1Turn => is_p1,
            RoomState::Player2Turn => !is_p1,
            _ => false,
        }
    }
}

pub struct RoomManager {
    pub rooms: std::collections::HashMap<String, Room>,
}

impl RoomManager {
    pub fn new() -> Self {
        Self {
            rooms: std::collections::HashMap::new(),
        }
    }

    pub fn create_room(&mut self, name: String, creator: String) -> bool {
        if self.rooms.contains_key(&name) {
            return false;
        }
        self.rooms.insert(name, Room::new(creator));
        true
    }

    pub fn join_room(&mut self, name: &str, guest: String) -> bool {
        self.rooms.get_mut(name)
            .map(|r| r.add_guest(guest))
            .unwrap_or(false)
    }

    pub fn remove_room(&mut self, name: &str) -> bool {
        self.rooms.remove(name).is_some()
    }

    pub fn get_room(&self, name: &str) -> Option<&Room> {
        self.rooms.get(name)
    }

    pub fn get_room_mut(&mut self, name: &str) -> Option<&mut Room> {
        self.rooms.get_mut(name)
    }

    pub fn is_host(&self, room_name: &str, username: &str) -> bool {
        self.rooms.get(room_name)
            .map(|r| r.belongs_to == username)
            .unwrap_or(false)
    }
}

impl Default for RoomManager {
    fn default() -> Self {
        Self::new()
    }
}