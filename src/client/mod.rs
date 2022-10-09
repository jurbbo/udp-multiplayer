pub mod client;
pub mod datahandlers;
mod socketlistener;
mod socketsender;

use crate::client::client::Client;
use crate::client::datahandlers::playercreatedresponse::PlayerCreatedServerError;
use crate::client::datahandlers::structs::player::{PlayerCreatedResponseData, PlayerData};
use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

pub trait RequestEvents {
    fn on_data_push_action(&mut self, raw_data: Vec<u8>);
    fn on_data_push_received(&mut self, from_player: u8, raw_data: Vec<u8>);
    fn on_data_request(&mut self, raw_data: Vec<u8>);
    fn on_pong(&mut self, interval: Duration);
    fn on_player_created(
        &mut self,
        response: Result<PlayerCreatedResponseData, PlayerCreatedServerError>,
    );
    fn on_player_enter_push(&mut self, player: PlayerData);
    fn on_player_leave(&mut self, raw_data: Vec<u8>);
    fn on_error(&mut self);
    fn on_connection_state_change(&mut self, new_state: bool);
}

pub fn run<S: 'static>(client: &mut Client, events: Arc<Mutex<S>>)
where
    S: RequestEvents + Send + Sync,
{
    // please note, that channels must be initialized first, since other threads need
    // the channel rx and tx attributes.
    client.init_channels();
    client.init_sender();
    client.init_job_handler();
    client.init_job_channel_consumer();
    client.init_listeners(events);
    // started -->
    client.mark_client_start();
}
