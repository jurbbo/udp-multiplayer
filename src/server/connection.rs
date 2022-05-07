use std::collections::HashMap;
use std::net::SocketAddr;

pub struct Connection {
    pub connections_count: i32,
    pub bytes_received: i128,
    pub bytes_send: i128,
    pub ping: u16,
    pub player_number: u8,
    pub player_name: String,
}

impl Connection {
    pub fn new(player_number: u8, player_name: String) -> Connection {
        Connection {
            connections_count: 0,
            bytes_received: 0,
            bytes_send: 0,
            ping: 0,
            player_name: player_name,
            player_number: player_number,
        }
    }
}

pub struct Connections {
    pub connections: HashMap<SocketAddr, Connection>,
}

impl Connections {
    pub fn new() -> Connections {
        Connections {
            connections: HashMap::<SocketAddr, Connection>::new(),
        }
    }

    pub fn create_new_connection(&mut self, ip: SocketAddr, player_name: String) -> bool {
        for index in 1..=255 {
            let mut is_index_found = false;
            for (_ip, connection) in &self.connections {
                if connection.player_number == index {
                    is_index_found = true;
                }
            }
            if !is_index_found {
                self.connections
                    .insert(ip, Connection::new(index, player_name));
                return true;
            }
        }
        false
    }
}
