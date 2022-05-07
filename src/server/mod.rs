pub mod connection;
pub mod server;

use crate::server::server::Server;

pub fn run() {
    let mut server = Server::new(5);
    let result = server.connect("127.0.0.1:11111".to_string());
    if result.is_err() {
        println!("IP / port failed...");
        println!("{}", result.unwrap_err());
        return;
    }
    println!("Close server with ctrl-c...");
    server.run();
}

pub fn test_server() {
    println!("Server starting...");
    run();
}
