use crate::client;
use crate::client::client::Client;
use crate::client::datahandlers::playercreatedresponse::PlayerCreatedServerError;
use crate::client::datahandlers::structs::player::PlayerCreatedResponseData;
use crate::client::datahandlers::structs::player::PlayerData;
use crate::client::RequestEvents;
use crate::protocol::builders::RawDataBuilder;
use crate::protocol::datahelpers;
use crate::protocol::datastructure::StructuredData;
use crate::protocol::Protocol;
use crate::requests::ClientJob;
use std::io::stdin;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
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

    fn on_pong(&mut self, _time: Duration) {
        todo!()
    }
    fn on_data_push_received(&mut self, player: u8, raw_data: Vec<u8>) {
        let data_string = String::from_utf8_lossy(&raw_data);
        println!("Received from player {}: {}", player, data_string);
    }
    fn on_data_request(&mut self, _: std::vec::Vec<u8>) {
        todo!()
    }
    fn on_player_enter_push(&mut self, player: PlayerData) {
        println!(
            "Entered player {} with player number {}. Player addr {}",
            player.player_name,
            player.player_number,
            match player.addr {
                None => "Just shit...".to_string(),
                _ => player.addr.unwrap().to_string(),
            }
        );
    }
    fn on_player_created(
        &mut self,
        player_created_response_data_result: Result<
            PlayerCreatedResponseData,
            PlayerCreatedServerError,
        >,
    ) {
        match player_created_response_data_result {
            Err(e) => {
                print!("{}", e)
            }
            Ok(player_created_response_data) => {
                println!(
                    "Created player number {}. Your name is: {}\n",
                    player_created_response_data.player.player_number,
                    player_created_response_data.player.player_name
                );
                for other_player in player_created_response_data.others_players {
                    println!(
                        "Other player number {}. Name: {}. Ip: {} \n",
                        other_player.player_number,
                        other_player.player_name,
                        other_player.addr.unwrap().to_string()
                    );
                }
            }
        }
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
    println!("UDP Multiplayer client... for debug testing");

    let protocols = Protocol::new();

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

    let raw_name_request_data = datahelpers::create_player_request(&protocols, name_owned);

    let result = client.send_request(
        ClientJob::PlayerEnterRequest,
        &mut raw_name_request_data.unwrap(),
    );
    if result.is_err() {
        println!("Error.")
    }

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    println!("Data string: ");
    let mut data = String::new();
    stdin().read_line(&mut data).expect("Shit");

    while running.load(Ordering::SeqCst) {
        thread::sleep(pause_time_duration);
        if client.is_in_error_state() {
            println!("server is in error state...");
            thread::sleep(Duration::new(1, 0));
        }

        match stuff.trim().as_ref() {
            "q" => {
                println!("graceful shutdown...");
                break;
            }
            "a" => {
                let _data_owned = data.trim().to_owned().as_bytes().to_vec();

                //let _result = client.send_request(ClientJob::DataPushRequest, &mut data_owned);
            }
            _ => {}
        }
    }

    println!("Exiting...");
    client.die();
}
