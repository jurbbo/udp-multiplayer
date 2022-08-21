use std::net::SocketAddr;

pub struct PlayerData {
    pub player_name: String,
    pub player_number: u8,
    pub addr: Option<SocketAddr>,
}

pub struct PlayerCreatedResponseData {
    pub player: PlayerData,
    pub others_players: Vec<PlayerData>,
}

impl PlayerCreatedResponseData {
    pub fn new(player: PlayerData) -> PlayerCreatedResponseData {
        PlayerCreatedResponseData {
            player: player,
            others_players: vec![],
        }
    }
    pub fn add_other_player(&mut self, player: PlayerData) {
        self.others_players.push(player);
    }
}
