mod lib;
use lib::server::Server;

fn main() {
    let server = Server::new(String::from("0.0.0.0"), 17115);
    server.run();
}
