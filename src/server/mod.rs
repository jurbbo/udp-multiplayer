pub mod connection;
pub mod server;
pub mod socketlistener;

use crate::server::server::Server;

pub fn run(ip: String, port: String, thread_count: u8) {
    let mut server = Server::new(thread_count);
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
    run("localhost".to_string(), "11111".to_string(), 3);
}
