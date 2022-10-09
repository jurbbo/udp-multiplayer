use crate::client::datahandlers::playercreatedresponse::PlayerCreatedServerError;
use crate::client::datahandlers::structs::player::PlayerCreatedResponseData;
use crate::client::datahandlers::structs::player::PlayerData;
use crate::client::RequestEvents;
use crate::protocol::Protocol;
use crate::tests::common::init_test_environment;
use crate::tests::common::pause;
use crate::tests::common::testclient::create_client;
use crate::tests::common::testcounter::add_finished_count;
use std::time::Duration;

use std::sync::Arc;
use std::sync::Mutex;

use crate::client;
use crate::protocol::datahelpers;
use crate::requests::ClientJob;
struct Eventful {
    // player name will be stored here, if server returns correctly
    pub player: Option<PlayerData>,
    pub other_players: Vec<PlayerData>,
}

impl RequestEvents for Eventful {
    fn on_data_push_action(&mut self, _raw_data: Vec<u8>) {}
    fn on_pong(&mut self, _time: Duration) {}
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
        self.other_players.push(player);
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
                self.player = Some(player_created_response_data.player);
                for other_player in player_created_response_data.others_players {
                    self.other_players.push(other_player);
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
#[test]
pub fn test_create_player_request_with_name() {
    init_test_environment();

    let events = Arc::new(Mutex::new(Eventful {
        player: None,
        other_players: vec![],
    }));
    let protocols = Protocol::new();
    let mut client = create_client();
    let events_new = Arc::clone(&events);
    client::run(&mut client, events_new);
    let raw_name_request_data =
        datahelpers::create_player_request(&protocols, "Testing name".to_string());

    let result = client.send_request(
        ClientJob::PlayerEnterRequest,
        &mut raw_name_request_data.unwrap(),
    );
    if result.is_err() {
        add_finished_count();
        assert_eq!(true, false);
    }
    while events.lock().unwrap().player.is_none() {}

    let server_request_name = (*events)
        .lock()
        .unwrap()
        .player
        .as_ref()
        .unwrap()
        .player_name
        .clone();

    client.die();
    add_finished_count();

    assert_eq!(server_request_name, "Testing name".to_string());
}
