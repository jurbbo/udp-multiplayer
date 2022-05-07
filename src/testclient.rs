use crate::client;
use crate::client::client::Client;
use crate::client::RequestEvents;
use crate::requests::jobtype::{ClientJob, ServerJob};
use std::io::stdin;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

/*
impl RequestEvents for Client {
    fn handle_data(job_type: u8, raw_data: std::vec::Vec<u8>) {
        println!("Data");
    }
    fn handle_ping(_: std::time::Instant) {
        todo!()
    }&mut self&mut self
}
*/
/*
fn handle_data_cb(job_type: ClientJobType, raw_data: &mut [u8]) {
    match job_type {
        ClientJobType::DataPushRequest => {}
        ClientJobType::DataRequest => {}
        ClientJobType::Data => {}
        ClientJobType::PLAYER_ENTER => {}
        ClientJobType::PLAYER_LEAVE => {}
        ClientJobType::PING => {
            println!("Pong")
        }
    }
}
*/

struct Eventful {
    pub count: u128,
}

impl RequestEvents for Eventful {
    fn on_data_push_action(&mut self, raw_data: Vec<u8>) {
        println!(
            "data events {} {}.. count {}",
            raw_data[0], raw_data[1], self.count
        );
        self.count += 1;
    }

    fn on_pong(&mut self, time: Duration) {
        todo!()
    }
    fn on_data_push_received(&mut self, player: u8, raw_data: Vec<u8>) {
        println!("Received from player {}", player);
    }
    fn on_data_request(&mut self, _: std::vec::Vec<u8>) {
        todo!()
    }
    fn on_player_enter(&mut self, player: u8, raw_data: std::vec::Vec<u8>) {
        println!("Entered player number {}", player);
    }
    fn on_player_created(&mut self, player: u8, raw_data: std::vec::Vec<u8>) {
        println!("Created player number {}", player);
    }

    fn on_player_leave(&mut self, _: std::vec::Vec<u8>) {
        todo!()
    }
    fn on_error(&mut self) {
        println!("UDP ERROR!!!");
    }
    fn on_connection_state_change(&mut self, error_state: bool) {
        if error_state {
            println!("Connection is lost");
        }
        if !error_state {
            println!("COnnection established");
        }
    }
}

pub fn testclient() {
    println!();
    println!("UDP Rusting client...");

    let server_job = ServerJob::DataResponse;
    let client_job = ClientJob::DataPushRequest;

    let byte = crate::requests::jobtype::get_job_single_byte(&(server_job, client_job));
    println!("{:#b}", byte);

    let server = crate::protocol::datastructure::get_u8_from_bit_slice(byte, 0, 4);
    let client = crate::protocol::datastructure::get_u8_from_bit_slice(byte, 4, 4);

    println!("{},{}", server, client);

    /*
    let first: u8 = 0b10101110;
    let second: u8 = 0b11110010;
    let third: u8 = 0b10001111;

    let bytes = vec![first, second, third];

    let mut data_s = DataStructure::new(bytes);

    data_s.add_structure(
        "test1".to_string(),
        4,
        2,
        DataType::NUMBERDATA,
        "Testing 1, count 2 bits".to_string(),
    );

    data_s.add_structure(
        "test2".to_string(),
        6,
        2,
        DataType::NUMBERDATA,
        "Testing 2, count 2 bits".to_string(),
    );

    println!(
        "{}",
        data_s.get_u8_from_bit_slice("test1".to_string()).unwrap()
    );
    println!(
        "{}",
        data_s.get_u8_from_bit_slice("test2".to_string()).unwrap()
    );

    */

    println!("port: ");
    let mut local_ip: String = "127.0.0.1:".to_owned();
    let mut port = String::new();
    stdin().read_line(&mut port).expect("Shit happened...");
    local_ip.push_str(&port.trim());

    let mut client = Client::new(3);

    let result = client.connect(local_ip, "localhost:11111".to_string());
    if result.is_err() {
        println!("Connection failed");
        return;
    }

    let events = Arc::new(Mutex::new(Eventful { count: 0 }));
    client::run(&mut client, events);

    println!("Pause between data grams: ");
    let mut pause_time = String::new();
    stdin()
        .read_line(&mut pause_time)
        .expect("Shit happened...");

    let pause_time_duration =
        std::time::Duration::new(0, pause_time.trim().parse::<u32>().unwrap() * 1000000);

    println!("Select stuff to send: a / s: ");
    let mut stuff = String::new();
    stdin().read_line(&mut stuff).expect("Shit happened...");

    println!("Player name: ");
    let mut name = String::new();
    stdin().read_line(&mut name).expect("Shit");
    let name_owned = name.trim().to_owned();
    let mut name_bytes = name_owned.as_bytes().to_vec();

    let result = client.send_request(ClientJob::PlayerEnterRequest, &mut name_bytes);
    if result.is_err() {
        println!("Error.")
    }

    loop {
        if client.is_in_error_state() {
            //println!("Server is in error state...");
            //thread::sleep(Duration::new(1, 0));
        }

        thread::sleep(pause_time_duration);
        match stuff.trim().as_ref() {
            "q" => {
                println!("graceful fall...");
                break;
            }
            "a" => {
                let result = client.send_request(ClientJob::DataPushRequest, &mut vec![3, 4, 5, 6]);
            }
            "s" => {
                let result = client.send_request(ClientJob::PlayerEnterRequest, &mut name_bytes);
            }

            _ => {}
        }
    }

    println!("Exiting...");

    client.die();
}
