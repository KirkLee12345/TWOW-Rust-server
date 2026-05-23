use std::fmt::format;
use chrono::Local;
use std::fs;
use std::fs::OpenOptions;
use std::io::{Read, Write, BufReader};
use std::net::{TcpListener, TcpStream};
use std::ops::Add;
use std::thread;
use std::sync::atomic::{AtomicUsize, Ordering};
use crate::lib::game;
use game::handle_data;

pub(crate) const VERSION: &str = "1.2.3";
pub(crate) const PROTOCOL_VERSION: i32 = 3;
pub(crate) const IS_DEBUG: bool = true;
pub(crate) const FROM_EMAIL: &str = "TDR_Group@foxmail.com";
pub(crate) const SLPPE_TIME_MILLIS: u64 = 10;

static THREAD_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub fn llog(message: String) -> Result<(), std::io::Error> {
    let date_str = Local::now().format("%Y-%m-%d").to_string();
    let time_str = Local::now().format("%H:%M:%S").to_string();
    let mut file = OpenOptions::new()
        .create(true)     // 文件不存在则创建
        .append(true)     // 以追加模式打开
        .open(format!("logs/{date_str}.txt"))?;
    println!("[{date_str} {time_str}] {message}");
    file.write_all(format!("[{date_str} {time_str}] {message}\n").as_bytes())?;
    Ok(())
}

pub fn log(message: String) {
    llog(message).expect("错误: 写入日志失败。");
}

pub struct Server {
    version: String,
    protocol_version: i32,
    host: String,
    port: u16,
}

impl Server {
    pub fn new(host: String, port: u16) -> Server {
        Server{
            version: VERSION.to_string(),
            protocol_version: PROTOCOL_VERSION,
            host,
            port
        }
    }
    pub fn run(&self) {
        let logs_dir = std::path::Path::new("logs");
        if !logs_dir.exists() {fs::create_dir_all(logs_dir).expect("错误: 无法创建 logs 目录。");}
        fs::read_to_string("email.key").expect("错误: 无法读取 email.key 文件，请确保该文件存在于程序运行目录。");

        let listener = TcpListener::bind(format!("{}:{}", self.host, self.port)).unwrap();
        log(format!("服务端启动完成，版本号：{}，协议版本：{}，监听地址：{}:{}", self.version, self.protocol_version, self.host, self.port));
        if IS_DEBUG {
            log("已开启DEBUG模式，所有交互数据将打印到控制台".to_string());
        }
        for connection in listener.incoming() {
            let client = connection.unwrap();
            let thread_index = THREAD_COUNTER.fetch_add(1, Ordering::SeqCst);
            thread::spawn(move || {
                handle_connection(client, thread_index);
            });
        }
    }
}

fn handle_connection(stream: TcpStream, thread_index: usize) {
    let read_stream = stream.try_clone().expect(format!("[{thread_index}] 克隆 read_stream 失败").as_str());
    let mut write_stream = stream.try_clone().expect(format!("[{thread_index}] 克隆 write_stream 失败").as_str());
    let mut reader = BufReader::new(read_stream);
    let mut buffer = [0u8; 1024];

    let peer_addr = stream.peer_addr().expect("无法获取客户端地址");
    log(format!("[{thread_index}] 开始处理来自 {} 的连接", peer_addr));

    let mut is_login = false;
    let mut zh = "".to_string();

    loop {
        match reader.read(&mut buffer) {
            Ok(0) => {
                log(format!("[{thread_index}] 连接关闭"));
                break;
            }
            Ok(n) => {
                let message = String::from_utf8_lossy(&buffer[..n]).to_string();
                if !message.starts_with("f**k") && IS_DEBUG{
                    log(format!("[{thread_index}] 收到数据: {message}"));
                }
                let response = handle_data(message, thread_index, &mut is_login, &mut zh, stream.try_clone().expect(format!("[{thread_index}] 克隆 stream 失败").as_str()));
                if response != "null"{
                    if !response.starts_with("f**k") && IS_DEBUG {
                        log(format!("[{thread_index}] 发送数据: {response}"));
                    }
                    write_stream.write_all(response.as_bytes()).expect("发送错误");
                }
            }
            Err(e) => {
                log(format!("[{thread_index}] 读取错误: {e}"));
                break;
            }
        }
    }
}
