pub mod client;
mod socketlistener;

use crate::client::client::Client;
use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

pub trait RequestEvents {
    fn on_data_push_action(&mut self, raw_data: Vec<u8>);
    fn on_data_push_received(&mut self, from_player: u8, raw_data: Vec<u8>);
    fn on_data_request(&mut self, raw_data: Vec<u8>);
    fn on_pong(&mut self, interval: Duration);
    fn on_player_created(&mut self, player_number: u8, raw_data: Vec<u8>);
    fn on_player_enter(&mut self, player_number: u8, raw_data: Vec<u8>);
    fn on_player_leave(&mut self, raw_data: Vec<u8>);
    fn on_error(&mut self);
    fn on_connection_state_change(&mut self, new_state: bool);
}

pub fn run<S: 'static>(client: &mut Client, events: Arc<Mutex<S>>)
where
    S: RequestEvents + Send + Sync,
{
    client.init_job_handler();
    client.init_listeners(events);
}
