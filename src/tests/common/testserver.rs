use crate::server;
use crate::server::server::Server;
use crate::socket::SocketCombatible;
use crate::tests::common::pause;
use std::borrow::BorrowMut;
use std::sync::Mutex;
use std::sync::Once;

static mut SERVER: Option<Mutex<Server>> = None;
static INITSERVER: Once = Once::new();

pub fn global_test_server<'a>() -> &'a Mutex<Server> {
    INITSERVER.call_once(|| {
        for port in 49152..65535 {
            match server::run("localhost".to_string(), port.to_string(), 5) {
                Err(e) => {
                    println!("{}", e);
                }
                Ok(server) => unsafe {
                    println!("server running on port {}", port);
                    *SERVER.borrow_mut() = Some(Mutex::new(server));
                    break;
                }, // Since this access is inside a call_once, before any other accesses, it is safe
            }
        }
    });
    // As long as this function is the only place with access to the static variable,
    // giving out a read-only borrow here is safe because it is guaranteed no more mutable
    // references will exist at this point or in the future.
    unsafe { SERVER.as_ref().unwrap() }
}

pub fn close_server() {
    global_test_server().lock().unwrap().die();
}

pub fn is_server_running() -> bool {
    global_test_server().lock().unwrap().is_running()
}

pub fn wait_till_server_is_running() {
    while !is_server_running() {
        pause(250);
    }
}

pub fn get_server_port() -> Option<u16> {
    global_test_server().lock().unwrap().get_port()
}
