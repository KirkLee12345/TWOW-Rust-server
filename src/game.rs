use crate::room::{Room, RoomState, Player};

pub struct GameLogic;

fn give_card_to_player(player: &mut Player, card_opt: Option<String>) -> bool {
    let Some(card) = card_opt else { return false; };
    for slot in player.hand_cards.iter_mut() {
        if *slot == "0" {
            *slot = card;
            return true;
        }
    }
    false
}

impl GameLogic {
    pub fn pass_card(room: &mut Room, player_index: usize, card_index: usize) -> PassResult {
        let player = if player_index == 1 { &mut room.player1 } else { &mut room.player2 };
        let card = player.hand_cards.get(card_index).cloned().unwrap_or_default();
        
        if card == "0" {
            return PassResult::Error("参数错误(该手牌不存在)".to_string());
        }
        
        let valid_pass_cards = vec!["d0", "g0", "k0", "n0", "w2", "w4"];
        if !valid_pass_cards.contains(&card.as_str()) {
            return PassResult::Error("参数错误(该手牌不能作为被动卡牌)".to_string());
        }
        
        for slot in player.passive_cards.iter_mut() {
            if *slot == "0" {
                *slot = card;
                player.hand_cards[card_index] = "0".to_string();
                return PassResult::Success;
            }
        }
        
        PassResult::Error("参数错误(被动卡槽已满)".to_string())
    }

    pub fn use_card(room: &mut Room, player_index: usize, card_index: usize) -> UseCardResult {
        let card = if player_index == 1 {
            room.player1.hand_cards.get(card_index).cloned().unwrap_or_default()
        } else {
            room.player2.hand_cards.get(card_index).cloned().unwrap_or_default()
        };
        
        if card == "0" {
            return UseCardResult::Error("参数错误(该手牌不存在)".to_string());
        }
        
        match card.as_str() {
            "w2" => {
                if player_index == 1 {
                    room.player1.hand_cards[card_index] = "0".to_string();
                } else {
                    room.player2.hand_cards[card_index] = "0".to_string();
                }
                room.last_card = card.clone();
                if player_index == 1 {
                    room.player1.used = true;
                } else {
                    room.player2.used = true;
                }
                for _ in 0..2 {
                    let card_opt = Self::draw_card(room);
                    if player_index == 1 {
                        give_card_to_player(&mut room.player1, card_opt);
                    } else {
                        give_card_to_player(&mut room.player2, card_opt);
                    }
                }
                return UseCardResult::DrawCards(2);
            }
            "w4" => {
                if player_index == 1 {
                    room.player1.hand_cards[card_index] = "0".to_string();
                } else {
                    room.player2.hand_cards[card_index] = "0".to_string();
                }
                room.last_card = card.clone();
                if player_index == 1 {
                    room.player1.used = true;
                } else {
                    room.player2.used = true;
                }
                for _ in 0..4 {
                    let card_opt = Self::draw_card(room);
                    if player_index == 1 {
                        give_card_to_player(&mut room.player1, card_opt);
                    } else {
                        give_card_to_player(&mut room.player2, card_opt);
                    }
                }
                return UseCardResult::DrawCards(4);
            }
            _ => {}
        }
        
        let energy_cost = card[1..].parse::<i32>().unwrap_or(0);
        
        if card.starts_with('n') {
            if player_index == 1 {
                room.player1.energy += energy_cost;
            } else {
                room.player2.energy += energy_cost;
            }
            return UseCardResult::EnergyChange(energy_cost);
        }
        
        if card.starts_with('k') {
            let opponent = if player_index == 1 { 2 } else { 1 };
            if Self::trigger_passive(room, opponent, "k0") {
                return UseCardResult::EnergyReductionReduced;
            }
            if opponent == 1 {
                room.player1.energy -= energy_cost;
            } else {
                room.player2.energy -= energy_cost;
            }
            return UseCardResult::EnergyReduction(energy_cost);
        }
        
        if card.starts_with('d') {
            let player = if player_index == 1 { &mut room.player1 } else { &mut room.player2 };
            if player.energy < energy_cost {
                return UseCardResult::Error("参数错误(能量不足)".to_string());
            }
            player.energy -= energy_cost;
            
            let mut placed = false;
            for slot in player.out_cards.iter_mut() {
                if *slot == "0" {
                    *slot = card.clone();
                    placed = true;
                    break;
                }
            }
            
            if !placed {
                player.energy += energy_cost;
                return UseCardResult::Error("参数错误(盾牌槽已满)".to_string());
            }
            
            if player_index == 1 {
                room.player1.hand_cards[card_index] = "0".to_string();
            } else {
                room.player2.hand_cards[card_index] = "0".to_string();
            }
            room.last_card = card;
            if player_index == 1 {
                room.player1.used = true;
            } else {
                room.player2.used = true;
            }
            return UseCardResult::Defend;
        }
        
        if card.starts_with('g') {
            let player = if player_index == 1 { &mut room.player1 } else { &mut room.player2 };
            if player.energy < energy_cost {
                return UseCardResult::Error("参数错误(能量不足)".to_string());
            }
            player.energy -= energy_cost;
            
            let damage = energy_cost;
            let opponent_index = if player_index == 1 { 1 } else { 2 };
            let opponent = if opponent_index == 1 { &mut room.player1 } else { &mut room.player2 };
            
            let (damage_after_shield, blocked) = Self::calculate_damage_internal(opponent, damage);
            
            if damage_after_shield > 0 {
                for _ in 0..damage_after_shield {
                    Self::remove_random_card_from_player(opponent);
                }
            }
            
            if player_index == 1 {
                room.player1.hand_cards[card_index] = "0".to_string();
            } else {
                room.player2.hand_cards[card_index] = "0".to_string();
            }
            room.last_card = card;
            if player_index == 1 {
                room.player1.used = true;
            } else {
                room.player2.used = true;
            }
            
            return UseCardResult::Attack { damage, blocked };
        }
        
        if player_index == 1 {
            room.player1.hand_cards[card_index] = "0".to_string();
        } else {
            room.player2.hand_cards[card_index] = "0".to_string();
        }
        room.last_card = card;
        if player_index == 1 {
            room.player1.used = true;
        } else {
            room.player2.used = true;
        }
        UseCardResult::Played
    }

    fn draw_card(room: &mut Room) -> Option<String> {
        if room.all_cards.is_empty() {
            return None;
        }
        let idx = {
            let seed = std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(42);
            ((seed as usize).wrapping_mul(1103515245).wrapping_add(12345)) % room.all_cards.len()
        };
        Some(room.all_cards.remove(idx))
    }

    fn remove_random_card_from_player(player: &mut Player) {
        let available: Vec<(usize, String)> = player.hand_cards.iter()
            .enumerate()
            .filter(|(_, c)| **c != "0")
            .map(|(i, c)| (i, c.clone()))
            .collect();
        
        if available.is_empty() {
            return;
        }
        
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(42);
        let idx = ((seed as usize).wrapping_mul(1103515245).wrapping_add(12345)) % available.len();
        let pos = available[idx].0;
        player.hand_cards[pos] = "0".to_string();
    }

    fn trigger_passive(room: &mut Room, player_index: usize, card: &str) -> bool {
        let player = if player_index == 1 { &mut room.player1 } else { &mut room.player2 };
        if let Some(pos) = player.passive_cards.iter().position(|c| c == card) {
            player.passive_cards[pos] = "0".to_string();
            return true;
        }
        false
    }

    fn calculate_damage_internal(player: &mut Player, damage: i32) -> (i32, bool) {
        let mut remaining_damage = damage;
        let mut blocked = false;
        
        for slot in player.out_cards.iter_mut() {
            if *slot == "0" || remaining_damage <= 0 {
                continue;
            }
            
            let shield_value = slot[1..].parse::<i32>().unwrap_or(0);
            
            if shield_value == 1 {
                *slot = "0".to_string();
                blocked = true;
                remaining_damage = 0;
                break;
            } else if shield_value > remaining_damage {
                *slot = format!("d{}", shield_value - remaining_damage);
                blocked = true;
                remaining_damage = 0;
                break;
            } else {
                remaining_damage -= shield_value;
                *slot = "0".to_string();
                blocked = true;
            }
        }
        
        (remaining_damage, blocked)
    }

    pub fn next_turn(room: &mut Room) {
        match room.now {
            RoomState::Player1Turn => {
                room.now = RoomState::Player2Turn;
                if room.player2.energy < 6 {
                    room.player2.energy = (room.player2.energy + 2).min(6);
                }
                if !room.player2.used {
                    let card_opt = Self::draw_card(room);
                    give_card_to_player(&mut room.player2, card_opt);
                }
                room.player2.used = false;
            }
            RoomState::Player2Turn => {
                room.now = RoomState::Player1Turn;
                if room.player1.energy < 6 {
                    room.player1.energy = (room.player1.energy + 2).min(6);
                }
                if !room.player1.used {
                    let card_opt = Self::draw_card(room);
                    give_card_to_player(&mut room.player1, card_opt);
                }
                room.player1.used = false;
            }
            _ => {}
        }
    }

    pub fn check_game_end(room: &mut Room) -> Option<GameResult> {
        let p1_hand_count = room.player1.hand_cards.iter().filter(|c| **c != "0").count();
        let p2_hand_count = room.player2.hand_cards.iter().filter(|c| **c != "0").count();
        
        if p1_hand_count == 0 {
            return Some(Self::trigger_death_effect(room, 1));
        }
        
        if p2_hand_count == 0 {
            return Some(Self::trigger_death_effect(room, 2));
        }
        
        None
    }

    fn trigger_death_effect(room: &mut Room, player_index: usize) -> GameResult {
        if player_index == 1 {
            if let Some(pos) = room.player1.passive_cards.iter().position(|c| *c == "w2") {
                room.player1.passive_cards[pos] = "0".to_string();
                for _ in 0..2 {
                    let card_opt = Self::draw_card(room);
                    give_card_to_player(&mut room.player1, card_opt);
                }
                return GameResult::Continue;
            }
            
            if let Some(pos) = room.player1.passive_cards.iter().position(|c| *c == "w4") {
                room.player1.passive_cards[pos] = "0".to_string();
                for _ in 0..4 {
                    let card_opt = Self::draw_card(room);
                    give_card_to_player(&mut room.player1, card_opt);
                }
                return GameResult::Continue;
            }
            return GameResult::Player2Win;
        } else {
            if let Some(pos) = room.player2.passive_cards.iter().position(|c| *c == "w2") {
                room.player2.passive_cards[pos] = "0".to_string();
                for _ in 0..2 {
                    let card_opt = Self::draw_card(room);
                    give_card_to_player(&mut room.player2, card_opt);
                }
                return GameResult::Continue;
            }
            
            if let Some(pos) = room.player2.passive_cards.iter().position(|c| *c == "w4") {
                room.player2.passive_cards[pos] = "0".to_string();
                for _ in 0..4 {
                    let card_opt = Self::draw_card(room);
                    give_card_to_player(&mut room.player2, card_opt);
                }
                return GameResult::Continue;
            }
            return GameResult::Player1Win;
        }
    }

    pub fn build_room_info(room: &Room, for_player: &str) -> String {
        let is_p1 = room.belongs_to == for_player;
        let mut info = String::from("game nowinfo");
        
        if is_p1 {
            for card in &room.player1.hand_cards {
                info.push_str(&format!(" {}", card));
            }
            for card in &room.player1.passive_cards {
                info.push_str(&format!(" {}", card));
            }
            for card in &room.player2.hand_cards {
                if card == "0" {
                    info.push_str(" 0");
                } else {
                    info.push_str(" b");
                }
            }
            for card in &room.player2.passive_cards {
                if card == "0" {
                    info.push_str(" 0");
                } else {
                    info.push_str(" b");
                }
            }
            info.push_str(&format!(" {}", if room.all_cards.is_empty() { "0" } else { "b" }));
            for card in &room.player1.out_cards {
                info.push_str(&format!(" {}", card));
            }
            for card in &room.player2.out_cards {
                info.push_str(&format!(" {}", card));
            }
            info.push_str(&format!(" {} {}", room.player1.energy, room.player2.energy));
            info.push_str(&format!(" {}", room.all_cards.len()));
            info.push_str(&format!(" {}", if room.now == RoomState::Player1Turn { "1" } else { "0" }));
        } else {
            for card in &room.player2.hand_cards {
                info.push_str(&format!(" {}", card));
            }
            for card in &room.player2.passive_cards {
                info.push_str(&format!(" {}", card));
            }
            for card in &room.player1.hand_cards {
                if card == "0" {
                    info.push_str(" 0");
                } else {
                    info.push_str(" b");
                }
            }
            for card in &room.player1.passive_cards {
                if card == "0" {
                    info.push_str(" 0");
                } else {
                    info.push_str(" b");
                }
            }
            info.push_str(&format!(" {}", if room.all_cards.is_empty() { "0" } else { "b" }));
            for card in &room.player2.out_cards {
                info.push_str(&format!(" {}", card));
            }
            for card in &room.player1.out_cards {
                info.push_str(&format!(" {}", card));
            }
            info.push_str(&format!(" {} {}", room.player2.energy, room.player1.energy));
            info.push_str(&format!(" {}", room.all_cards.len()));
            info.push_str(&format!(" {}", if room.now == RoomState::Player2Turn { "1" } else { "0" }));
        }
        
        info.push_str(&format!(" {}", room.last_card));
        info
    }
}

#[derive(Debug)]
pub enum PassResult {
    Success,
    Error(String),
}

#[derive(Debug)]
pub enum UseCardResult {
    Success,
    Error(String),
    DrawCards(u8),
    EnergyChange(i32),
    EnergyReduction(i32),
    EnergyReductionReduced,
    Defend,
    Attack { damage: i32, blocked: bool },
    Played,
}

#[derive(Debug)]
pub enum GameResult {
    Continue,
    Player1Win,
    Player2Win,
    Draw,
}