use crate::client::client::Client;
use crate::tests::common::testserver::get_server_port;

pub fn create_client() -> Client {
    let mut client = Client::new(3);
    let server_port = get_server_port().unwrap();
    for port in 49152..65535 {
        let local_ip: String = format!("127.0.0.1:{}", port);
        let result = client.connect(local_ip, format!("localhost:{}", server_port));
        match result {
            Err(_e) => {}
            Ok(_r) => {
                println!("Client using port {}", port);
                break;
            }
        }
    }
    client
}
