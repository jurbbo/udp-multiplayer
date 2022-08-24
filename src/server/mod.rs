pub mod connection;
pub mod server;
pub mod socketlistener;

use crate::server::server::Server;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{thread, time};

pub fn run(ip: String, port: String, thread_count: u8) -> Result<Server, std::io::Error> {
    let mut server = Server::new(thread_count);
    server.connect(ip + ":" + &port)?;
    server.run();
    Ok(server)
}

pub fn test_server() {
    println!("Server starting...");
    match run("localhost".to_string(), "11111".to_string(), 3) {
        Err(e) => {
            println!("Server error: {}", e);
        }
        Ok(mut server) => {
            // Test server will close with Ctrl-c. Let's create a loop.
            let running = Arc::new(AtomicBool::new(true));
            let r = running.clone();
            ctrlc::set_handler(move || {
                r.store(false, Ordering::SeqCst);
            })
            .expect("Error setting Ctrl-C handler");
            while running.load(Ordering::SeqCst) {
                thread::sleep(time::Duration::from_millis(500));
            }
            println!("");
            println!("Graceful shutdown...");
            match server.die() {
                true => println!("Gracefully closed!"),
                false => println!("Failed. Disgrace!"),
            }
        }
    }
}
