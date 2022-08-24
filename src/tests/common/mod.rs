use crate::client::client::Client;
use crate::server;
use crate::server::server::Server;
use std::borrow::BorrowMut;
use std::sync::Mutex;
use std::sync::Once;
use std::thread;
use std::time::Duration;

static mut SERVER: Option<Mutex<Server>> = None;
static INIT: Once = Once::new();

pub fn global_server<'a>() -> &'a Mutex<Server> {
    INIT.call_once(|| {
        for port in 49152..65535 {
            match server::run("localhost".to_string(), port.to_string(), 5) {
                Err(e) => {
                    panic!("{}", e);
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

pub fn close_server() -> bool {
    global_server().lock().unwrap().die();
    global_server().lock().unwrap().is_running()
}

pub fn wait_till_is_running() {
    while global_server().lock().unwrap().is_running() == false {
        thread::sleep(Duration::new(1, 0));
    }
}

pub fn get_server_port() -> Option<u16> {
    global_server().lock().unwrap().get_port()
}

pub fn create_client() -> Client {
    let mut client = Client::new(3);
    let server_port = get_server_port().unwrap();
    for port in 49152..65535 {
        let local_ip: String = format!("127.0.0.1:{}", port);
        let result = client.connect(local_ip, format!("localhost:{}", server_port));
        match result {
            Err(e) => {
                println!("Connection failed, port {} -- {}", port, e);
            }
            Ok(_r) => {
                println!("Client using port {}", port);
                break;
            }
        }
    }
    client
}

pub fn pause(time: u64) {
    let ten_millis = Duration::from_millis(time);
    thread::sleep(ten_millis);
}
