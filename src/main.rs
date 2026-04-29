mod server;
mod user;
mod room;
mod game;
mod email;
mod data;

use server::Server;
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    std::fs::create_dir_all("logs").ok();
    println!("TWOW Server starting...");

    let server = Arc::new(Mutex::new(Server::new("0.0.0.0".to_string(), 17115)));
    server.lock().unwrap().load();

    let srv = server.clone();
    let _ = thread::spawn(move || {
        srv.lock().unwrap().run();
    });

    println!("Press Enter to stop server...");
    let _ = std::io::stdin().read_line(&mut String::new());
    println!("TWOW Server shutting down...");
}