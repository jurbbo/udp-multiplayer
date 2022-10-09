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
pub fn test_create_player_request_other_player_names() {
    init_test_environment();
    let events1 = Arc::new(Mutex::new(Eventful {
        player: None,
        other_players: vec![],
    }));
    let events2 = Arc::new(Mutex::new(Eventful {
        player: None,
        other_players: vec![],
    }));
    let events3 = Arc::new(Mutex::new(Eventful {
        player: None,
        other_players: vec![],
    }));
    let protocols = Protocol::new();
    let mut client1 = create_client();
    let mut client2 = create_client();
    let mut client3 = create_client();
    let events_1 = Arc::clone(&events1);
    client::run(&mut client1, events_1);
    let events_2 = Arc::clone(&events2);
    client::run(&mut client2, events_2);
    let events_3 = Arc::clone(&events3);
    client::run(&mut client3, events_3);

    let raw_name_request_data1 =
        datahelpers::create_player_request(&protocols, "Tester 1".to_string());

    let raw_name_request_data2 =
        datahelpers::create_player_request(&protocols, "Tester 2".to_string());

    let raw_name_request_data3 =
        datahelpers::create_player_request(&protocols, "Tester 3".to_string());

    let result1 = client1.send_request(
        ClientJob::PlayerEnterRequest,
        &mut raw_name_request_data1.unwrap(),
    );
    if result1.is_err() {
        add_finished_count();
        assert_eq!(true, false);
    }

    let result2 = client2.send_request(
        ClientJob::PlayerEnterRequest,
        &mut raw_name_request_data2.unwrap(),
    );
    if result2.is_err() {
        add_finished_count();
        assert_eq!(true, false);
    }

    let result3 = client3.send_request(
        ClientJob::PlayerEnterRequest,
        &mut raw_name_request_data3.unwrap(),
    );
    if result3.is_err() {
        add_finished_count();
        assert_eq!(true, false);
    }

    loop {
        pause(100);
        if check_for_players(&events1, &events2, &events3) {
            break;
        }
    }

    let other_players = &(*events1).lock().unwrap().other_players;

    let has_names_for_events1 = check_other_player_names(other_players, "Tester 1".to_string())
        && check_other_player_names(other_players, "Tester 2".to_string())
        && check_other_player_names(other_players, "Tester 3".to_string());
    drop(other_players);

    let other_players = &(*events2).lock().unwrap().other_players;
    let has_names_for_events2 = check_other_player_names(other_players, "Tester 1".to_string())
        && check_other_player_names(other_players, "Tester 2".to_string())
        && check_other_player_names(other_players, "Tester 3".to_string());
    drop(other_players);

    let other_players = &(*events3).lock().unwrap().other_players;
    let has_names_for_events3 = check_other_player_names(other_players, "Tester 1".to_string())
        && check_other_player_names(other_players, "Tester 2".to_string())
        && check_other_player_names(other_players, "Tester 3".to_string());
    drop(other_players);

    // does not work.
    //client1.die();
    //client2.die();
    //client3.die();

    add_finished_count();

    assert_eq!(
        true,
        has_names_for_events1 && has_names_for_events2 && has_names_for_events3
    );
}

fn check_for_players(
    events1: &Arc<Mutex<Eventful>>,
    events2: &Arc<Mutex<Eventful>>,
    events3: &Arc<Mutex<Eventful>>,
) -> bool {
    if events1.lock().unwrap().player.is_some()
        && events2.lock().unwrap().player.is_some()
        && events3.lock().unwrap().player.is_some()
    {
        return true;
    }
    false
}

fn check_other_player_names(data: &Vec<PlayerData>, name: String) -> bool {
    if data.iter().any(|player| player.player_name == name) {
        return true;
    }
    false
}
