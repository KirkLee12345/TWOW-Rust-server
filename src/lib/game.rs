use std::io::Write;
use std::net::TcpStream;
use std::sync::{Mutex, OnceLock};
use std::thread;
use crate::lib::sign::send_yzm;
use crate::lib::users::{is_email_exist, is_user_exist, load_user_data, load_user_info, save_user_data, save_user_info, UserData, UserInfo};
use sha2::{Sha256, Digest};
use crate::lib::server::{log, PROTOCOL_VERSION};
use std::time::Duration;
use crate::lib::room;

pub(crate) static USERNAME_SOCKET_ROOM: OnceLock<Mutex<Vec<(String, TcpStream)>>> = OnceLock::new();
pub(crate) static EMA_YZM: OnceLock<Mutex<Vec<(String, String)>>> = OnceLock::new();


pub(crate) fn get_online_users() -> &'static Mutex<Vec<(String, TcpStream)>> {
    USERNAME_SOCKET_ROOM.get_or_init(|| Mutex::new(vec![]))
}
fn get_ema_yzm() -> &'static Mutex<Vec<(String, String)>> {
    EMA_YZM.get_or_init(|| Mutex::new(vec![]))
}


fn delete_room_by_belongs(user: &String) {
    let mut rooms = room::get_rooms().lock().unwrap();
    if let Some(pos) = rooms.iter().position(|r| r.name == *user) {
        rooms.remove(pos);
    }
}


pub fn get_client_by_user_name(user: &String) -> Option<TcpStream> {
    for (k, v) in get_online_users().lock().unwrap().iter() {
        if *k == *user {
            return Some(v.try_clone().unwrap());
        }
    }
    None
}


fn hash(s: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(s);
    let result = hasher.finalize();
    format!("{:x}", result)
}

pub fn handle_data(data: String, thread_index: usize, is_login: &mut bool, zh: &mut String, client: TcpStream) -> String {
    let data: Vec<&str> = data.split(" ").filter(|s| !s.is_empty()).collect();

    match data[0] {

        "sign" => {
            match data.len() {
                3 => {
                    match data[1] {
                        "username" => {
                            if data[2].len() < 4 || data[2].len() > 16 {
                                return String::from("用户名长度不符合要求");
                            }
                            let user_info = load_user_info(data[2]).expect("内部错误：用户信息加载失败");
                            if let Some(_) = user_info {
                                return String::from("用户名已存在");
                            }
                            return String::from("*用户名可用*");
                        }
                        "ema" => {
                            let yzm = send_yzm(thread_index, data[2].to_string());
                            get_ema_yzm().lock().unwrap().push((data[2].to_string(), yzm));
                            return String::from("sand yzm sucess");
                        }
                        _ => return String::from("tip [E101]无法解析的数据 ")
                    }
                }
                6 => {
                    match data[1] {
                        "up" => {
                            if is_email_exist(data[4]).unwrap() { return String::from("此邮箱已被绑定!"); }
                            for (k, v) in get_ema_yzm().lock().unwrap().iter() {
                                if k == data[4] && v == data[5] {
                                    let user_info = UserInfo {
                                        password_hash: hash(data[3]),
                                        email: data[4].to_string(),
                                    };
                                    save_user_info(data[2], &user_info).expect("内部错误：用户信息保存失败");
                                    let user_data = UserData {
                                        money: 0,
                                    };
                                    save_user_data(data[2], &user_data).expect("内部错误：用户数据保存失败");
                                    log(format!("[{thread_index}] {} 使用邮箱 {} 注册成功", data[2], data[4]));
                                    return String::from("注册成功!");
                                }
                            }
                            return String::from("验证码错误!");
                        }
                        _ => return String::from("tip [E102]无法解析的数据 ")
                    }
                }
                _ => return String::from("tip [E103]参数错误(参数数量不对) ")
            }
        }

        "login" => {
            match data.len() {
                4 => {
                    match data[1].parse::<i32>() {
                        Ok(n) => {
                            match n {
                                PROTOCOL_VERSION => {
                                    for (k, _) in get_online_users().lock().unwrap().iter() {
                                        if *k == data[2].to_string() {
                                            return String::from("重复登陆!");
                                        }
                                    }
                                    if is_user_exist(data[2]).unwrap() {
                                        if load_user_info(data[2]).unwrap().unwrap().password_hash == hash(data[3]) {
                                            get_online_users().lock().unwrap().push((data[2].to_string(), client));
                                            log(format!("[{thread_index}] {} 登陆成功", data[2]));
                                            *is_login = true;
                                            *zh = data[2].to_string();
                                            return String::from("登陆成功!");
                                        } else {
                                            return String::from("账号密码错误!");
                                        }
                                    } else {
                                        return String::from("账号密码错误!");
                                    }
                                }
                                _ => return String::from(format!("loginfail {PROTOCOL_VERSION}" )),
                            }
                        }
                        Err(e) => return String::from("tip [E104]参数错误(不可能的参数) "),
                    }
                }
                _ => return String::from("tip [E105]参数错误(参数数量不对) ")
            }
        }

        "f**k" => {
            match data.len() {
                2 => {
                    return String::from(format!("f**k {} ", data[1]));
                }
                _ => return String::from("tip [E000]参数错误(参数数量不对) ")
            }
        }

        "selfinfo" => {
            if !*is_login { return String::from("tip [E106]参数错误(未登录) "); }
            let money = load_user_data(zh).unwrap().unwrap().money;
            let online_players = get_online_users().lock().unwrap().len();
            return String::from(format!("selfinfo {} {money} {online_players}", *zh));
        }

        "test" => {
            if !*is_login { return String::from("tip [E901]参数错误(未登录) "); }
            match data.len() {
                2 => {
                    match data[1] {
                        "moneyadd1" => {
                            let mut user_data = load_user_data(zh).unwrap().unwrap();
                            user_data.money += 10;
                            save_user_data(zh, &user_data).expect("内部错误：用户数据保存失败");
                            log(format!("[{thread_index}] {zh} 测试增加10金币"));
                            let money = load_user_data(zh).unwrap().unwrap().money;
                            let online_players = get_online_users().lock().unwrap().len();
                            return String::from(format!("selfinfo {} {money} {online_players} ", *zh));
                        }
                        _ => return String::from("tip [E902]参数错误(无法解析的数据) ")
                    }
                }
                _ => return String::from("tip [E903]参数错误(参数数量不对) ")
            }
        }

        "room" => {
            if !*is_login { return String::from("tip [E107]参数错误(未登录) "); }
            match data.len() {
                3 => {
                    match data[1] {
                        "create" => {
                            for k in room::get_rooms().lock().unwrap().iter() {
                                if k.belongs_to == *zh || k.guest == *zh {
                                    return String::from("tip [E108]参数错误(已经在房间里) ");
                                }
                                if k.name == data[2] {
                                    return String::from("tip [E109]参数错误(房间名已存在) ");
                                }
                            }
                            let mut user_data = load_user_data(zh).unwrap().unwrap();
                            if user_data.money < 100 { return String::from("tip [E110]参数错误(金币不足) "); }
                            user_data.money -= 100;
                            save_user_data(zh, &user_data).expect("内部错误：用户数据保存失败");
                            let mut room = room::Room::default();
                            room.name = data[2].to_string();
                            room.belongs_to = zh.to_string();
                            room::get_rooms().lock().unwrap().push(room);
                            log(format!("[{thread_index}] {zh} 创建了房间 {}", data[2]));
                            return String::from(format!("CreateRoomSucess {} ", data[2]));
                        }
                        "join" => {
                            for k in room::get_rooms().lock().unwrap().iter_mut() {
                                if k.belongs_to == *zh || k.guest == *zh {
                                    return String::from("tip [E111]参数错误(已经在房间里) ");
                                }
                                if k.name == data[2] {
                                    if k.guest == "" {
                                        k.guest = zh.clone();
                                        log(format!("[{thread_index}] {zh} 加入了 {} 的房间 {}",k.belongs_to , k.name));
                                        client.try_clone().unwrap().write_all(&format!("JoinRoomSucess {} ", k.name).as_bytes()).expect("内部错误：发送数据失败");
                                        thread::sleep(Duration::from_millis(100));
                                        for (user, cc) in get_online_users().lock().unwrap().iter() {
                                            if *user == k.belongs_to {
                                                cc.try_clone().unwrap().write_all(&format!("game start {} ", *zh).as_bytes()).expect("内部错误：发送数据失败");
                                                break;
                                            }
                                        }
                                        return String::from(format!("game start {} ", k.belongs_to));
                                    } else {
                                        return String::from("tip [E112]参数错误(房间已满) ");
                                    }
                                }
                            }
                            return String::from("tip [E113]参数错误(房间不存在) ");
                        }
                        _ => return String::from("tip [E114]参数错误(无法解析的数据) ")
                    }
                }
                2 => {
                    match data[1] {
                        "r" => {
                            let mut r = String::from(format!("nowrooms {} ", room::get_rooms().lock().unwrap().len()));
                            for k in room::get_rooms().lock().unwrap().iter() {
                                r.push_str(k.name.as_str());
                                if k.guest == "" {
                                    r.push_str("┄(1/2)###");
                                } else {
                                    r.push_str("┄(2/2)###");
                                }
                            }
                            return r;
                        }
                        "exit" => {
                            let mut flag = false;
                            let mut room_other = String::from("");
                            let mut room_name = String::from("");
                            for k in room::get_rooms().lock().unwrap().iter_mut() {
                                if k.belongs_to == *zh {
                                    flag = true;
                                    room_other = k.guest.clone();
                                    room_name = k.name.clone();
                                    break;
                                }
                                if k.guest == *zh {
                                    flag = false;
                                    room_other = k.belongs_to.clone();
                                    room_name = k.name.clone();
                                    break;
                                }
                            }
                            log(format!("[{thread_index}] {zh} 退出了房间 {room_name}"));
                            if flag {
                                if room_other != "" {
                                    for (user, client) in get_online_users().lock().unwrap().iter_mut() {
                                        if *user == room_other {
                                            log(format!("[{thread_index}] 将访客 {user} 移出房间 {room_name}"));
                                            client.write_all("game exit ".as_bytes()).expect("发送错误");
                                            break;
                                        }
                                    }
                                }
                                delete_room_by_belongs(zh);
                                return String::from("tip 已退出房间 ");
                            } else {
                                if room_other != "" {
                                    for (user, client) in get_online_users().lock().unwrap().iter_mut() {
                                        if *user == room_other {
                                            log(format!("[{thread_index}] 将房主 {room_other} 移出房间 {room_name}"));
                                            client.write_all("game exit ".as_bytes()).expect("发送错误");
                                            break;
                                        }
                                    }
                                    delete_room_by_belongs(&room_other);
                                    return String::from("tip 已退出房间 ");
                                } else {
                                    return String::from("tip [E115]参数错误(未加入任何房间) ");
                                }
                            }
                        }
                        _ => return String::from("tip [E116]参数错误(无法解析的数据) ")
                    }
                }
                _ => return String::from("tip [E117]参数错误(参数数量不对) ")
            }
        }

        "game" => {
            if !*is_login { return String::from("tip [E118]参数错误(未登录) "); }
            if !room::check_is_in_room_by_user(zh) { return String::from("tip [E119]参数错误(未加入任何房间) "); }

            match data[1] {
                "chat" => {
                    if data.len() < 3 { return String::from("tip [E120]参数错误(参数数量不对) ");}
                    let mut r = "".to_string();
                    for i in 2..data.len() {
                        r.push_str(data[i]);
                        r.push(' ');
                    }
                    room::get_other_client_by_user(zh).unwrap().write_all(format!("game log {zh}:{r}").as_bytes()).expect("内部错误：发送数据失败");
                    log(format!("[{thread_index}] {zh} 在房间 {} 发送了消息 {r}", room::get_room_name_by_user(zh)));
                    return String::from(format!("game log {zh}:{r}"));
                }
                "start" => {
                    room::room_start(&room::get_room_name_by_user(zh));
                    return String::from("null");
                }
                "nowinfo" => {
                    return room::room_refresh(thread_index, zh);
                }
                "pass" => {
                    if !room::is_user_now_in_room(zh) { return String::from("tip [E121]参数错误(不是该玩家的回合) ");}
                    return room::room_pass(thread_index, zh, data[2].parse::<usize>().unwrap());
                }
                "next" => {
                    if !room::is_user_now_in_room(zh) { return String::from("tip [E122]参数错误(不是该玩家的回合) ");}
                    return room::room_next(thread_index, zh);
                }
                "use" => {
                    if !room::is_user_now_in_room(zh) { return String::from("tip [E123]参数错误(不是该玩家的回合) ");}
                    return room::room_use(thread_index, zh, data[2].parse::<usize>().unwrap());
                }
                _ => return String::from("tip [E   ]参数错误(无法解析的数据) ")
            }
        }

        _ => return String::from("tip [E   ]无法解析的数据")
    }

    unreachable!()
}