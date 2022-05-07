mod client;
mod params;
mod protocol;
mod requests;
mod server;
mod testclient;
use params::Params;

fn main() {
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    let mut params = Params::new();
    if params.has_param("server".to_string()) {
        println!("UDP Multiplayer Server -- version {}", VERSION);
        server::test_server();
    }
    if params.has_param("client".to_string()) {
        println!("UDP Multiplayer Client -- version {}", VERSION);
        testclient::testclient();
    }
    params.display_help();
}
