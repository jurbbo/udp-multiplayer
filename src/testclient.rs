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
use crate::requests::jobtype::ClientJob;
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
    println!("UDP Rusting client...");

    let protocols = Protocol::new();
    let player_created_response_protocol = protocols
        .get_protocol("PlayerCreatedResponse")
        .expect("No proto");
    protocols.print_protocol_structures("PlayerEnterPush");

    let structure = protocols
        .get_structures_ref("PlayerCreatedResponse")
        .expect("Not found");

    let mut raw_data_builder = RawDataBuilder::new(false)
        .add_vec_data("Status", structure, vec![30])
        .expect("Data builder error")
        .add_vec_data("PlayerNumber", structure, vec![30])
        .expect("Data builder error")
        .add_string_data("PlayerName", structure, "Moikka moi".to_string())
        .expect("Data builder error");

    let array_structure = protocols
        .get_array_structure_as_ref("PlayerCreatedResponse", "OtherPlayers")
        .expect("Array structure getter failed");

    for i in 1..11 {
        println!("{}", i);
        let name_string = match i {
            1 => "moi".to_string(),
            2 => "kakonen".to_string(),
            3 => "kolkki".to_string(),
            4 => "neli".to_string(),
            5 => "viiTonen tipiip".to_string(),
            6 => "kyrpönen".to_string(),
            7 => "seperi".to_string(),
            8 => "akrppi".to_string(),
            9 => "yrjökii".to_string(),
            10 => "pöö".to_string(),
            __ => "moi".to_string(),
        };

        let raw_data = RawDataBuilder::new(true)
            .add_vec_data("PlayerNumber", array_structure, vec![i * 10])
            .expect("Array builder failed on player number")
            .add_string_data("PlayerName", array_structure, name_string)
            .expect("Array builder failed on IP")
            .add_vec_data("PlayerIP", array_structure, vec![i, i, i, i])
            .expect("Array builder failed on IP")
            .add_u16_data("PlayerPort", array_structure, 10000)
            .expect("Array builder failed on IP")
            .get_raw_data();

        raw_data_builder = raw_data_builder
            .add_array_data("OtherPlayers", structure, raw_data)
            .expect("Array add failed");
    }

    let mut structured_data = StructuredData::new(
        player_created_response_protocol,
        raw_data_builder.get_raw_data(),
    );

    structured_data.print_protocol_structures();

    println!(
        "'{}'",
        structured_data.get_string_data("PlayerName").expect("Shit")
    );

    println!("{}", structured_data.raw_count());

    let mut indexing = 0;
    for array_item in structured_data
        .get_iterable_array("OtherPlayers")
        .expect("shit")
    {
        indexing += 1;
        print!(
            "{} -->>> {} ",
            indexing,
            array_item.get_string_data("PlayerName").expect("shit")
        );

        for letter in array_item
            .get_string_data("PlayerName")
            .expect("Shit")
            .chars()
        {
            print!("{}-", letter);
        }
        println!(
            "{}",
            array_item.get_vec_data("PlayerNumber").expect("Shit")[0]
        );
    }

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

    loop {
        if client.is_in_error_state() {
            //println!("Server is in error state...");
            //thread::sleep(Duration::new(1, 0));
        }

        thread::sleep(pause_time_duration);
        match stuff.trim().as_ref() {
            "q" => {
                println!("graceful shutdown...");
                break;
            }
            "a" => {
                println!("Data string: ");
                let mut data = String::new();
                stdin().read_line(&mut data).expect("Shit");
                let mut data_owned = data.trim().to_owned().as_bytes().to_vec();

                let _result = client.send_request(ClientJob::DataPushRequest, &mut data_owned);
            }
            _ => {}
        }
    }

    println!("Exiting...");

    client.die();
}
