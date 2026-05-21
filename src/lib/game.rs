use std::collections::HashMap;
use std::net::TcpStream;
use std::sync::{Mutex, OnceLock};
use serde::de::Unexpected::Option;
use crate::lib::sign::send_yzm;
use crate::lib::users::{is_email_exist, is_user_exist, load_email_yzm, load_user_data, load_user_info, save_email_yzm, save_user_data, save_user_info, UserData, UserInfo};
use sha2::{Sha256, Digest};
use crate::lib::server::{log, PROTOCOL_VERSION};

static INDEX_SOCKET_ROOM: OnceLock<Mutex<Vec<(String, TcpStream, String)>>> = OnceLock::new();
fn get_online_users() -> &'static Mutex<Vec<(String, TcpStream, String)>> {
    INDEX_SOCKET_ROOM.get_or_init(|| Mutex::new(vec![]))
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
                            save_email_yzm(data[2], yzm.as_str()).expect("内部错误：验证码保存失败");
                            return String::from("sand yzm sucess");
                        }
                        _ => return String::from("tip [E111]无法解析的数据")
                    }
                }
                6 => {
                    match data[1] {
                        "up" => {
                            if is_email_exist(data[4]).unwrap() { return String::from("此邮箱已被绑定!"); }
                            if load_email_yzm(data[4]).unwrap() == Some(data[5].to_string()) {
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
                            } else {
                                return String::from("验证码错误!");
                            }
                        }
                        _ => return String::from("tip [E112]无法解析的数据")
                    }
                }
                _ => return String::from("tip [E101]参数错误(参数数量不对)")
            }
        }

        "login" => {
            match data.len() {
                4 => {
                    match data[1].parse::<i32>() {
                        Ok(n) => {
                            match n {
                                PROTOCOL_VERSION => {
                                    for (k, _, _) in get_online_users().lock().unwrap().iter() {
                                        if *k == data[2].to_string() {
                                            return String::from("重复登陆!");
                                        }
                                    }
                                    if is_user_exist(data[2]).unwrap() {
                                        if load_user_info(data[2]).unwrap().unwrap().password_hash == hash(data[3]) {
                                            get_online_users().lock().unwrap().push((data[2].to_string(), client, "".to_string()));
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
                                _ => return String::from(format!("loginfail {PROTOCOL_VERSION}")),
                            }
                        }
                        Err(e) => return String::from("tip [E211]参数错误(不可能的参数)"),
                    }
                }
                _ => return String::from("tip [E201]参数错误(参数数量不对)")
            }
        }

        "f**k" => {
            match data.len() {
                2 => {
                    return String::from(format!("f**k {}", data[1]));
                }
                _ => return String::from("tip [E000]参数错误(参数数量不对)")
            }
        }

        "selfinfo" => {
            if !*is_login { return String::from("tip [E300]参数错误(未登录)"); }
            let money = load_user_data(zh).unwrap().unwrap().money;
            let online_players = get_online_users().lock().unwrap().len();
            return String::from(format!("selfinfo {} {money} {online_players}", *zh));
        }

        "test" => {
            if !*is_login { return String::from("tip [E900]参数错误(未登录)"); }
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
                            return String::from(format!("selfinfo {} {money} {online_players}", *zh));
                        }
                        _ => return String::from("tip [E911]参数错误(参数数量不对)")
                    }
                }
                _ => return String::from("tip [E901]参数错误(参数数量不对)")
            }
        }













        _ => return String::from("tip [E001]无法解析的数据")
    }

    unreachable!()
}