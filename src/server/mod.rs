pub mod connection;
pub mod server;
pub mod socketlistener;

use crate::server::server::Server;

pub fn run(ip: String, port: String) {
    let mut server = Server::new(5);
    let result = server.connect(ip + ":" + &port);
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
    run("127.0.0.1".to_string(), "11111".to_string());
}
