use crate::client::datahandlers::check_raw_data_length_integrity;
use crate::client::datahandlers::structs::player::{PlayerCreatedResponseData, PlayerData};
use crate::protocol::datahelpers::create_addr_from_ip_and_port;
use crate::protocol::datastructure::StructuredData;
use crate::protocol::{Protocol, ProtocolError};
use std::fmt::Display;
use std::fmt::Formatter;

#[derive(Debug)]
pub enum PlayerCreatedServerError {
    InvalidRequest = 100,
    NameIsTaken = 101,
    TooManyPlayers = 102,
    InvalidServerStatusCode,
}

impl Display for PlayerCreatedServerError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match *self {
            PlayerCreatedServerError::InvalidRequest => {
                write!(f, "Player created response: invalid request.")
            }
            PlayerCreatedServerError::NameIsTaken => {
                write!(f, "Player created response: name is taken.")
            }
            PlayerCreatedServerError::TooManyPlayers => {
                write!(f, "Player created response: too many players.")
            }
            PlayerCreatedServerError::InvalidServerStatusCode => {
                write!(f, "Player created response: invalid status code.")
            }
        }
    }
}

pub fn structurize_raw_data(
    protocols: &Protocol,
    raw_data: &[u8],
) -> Result<Result<PlayerCreatedResponseData, PlayerCreatedServerError>, ProtocolError> {
    let player_created_response_protocol = protocols.get_protocol("PlayerCreatedResponse")?;

    //check_raw_data_length_integrity(player_created_response_protocol, raw_data)?;

    let mut structured_data =
        StructuredData::new(player_created_response_protocol, raw_data.to_vec());

    let status = structured_data.get_u8_data("Status")?;

    // status 1 means that player is created succesfully
    if status == 1 {
        let player_number = structured_data.get_u8_data("PlayerNumber")?;
        let player_name = structured_data.get_string_data("PlayerName")?;
        let mut player_created_response_data = PlayerCreatedResponseData::new(PlayerData {
            player_name: player_name,
            player_number: player_number,
            addr: None,
        });

        // let's loop other player array data.
        for other_player_structured_data in structured_data.get_iterable_array("OtherPlayers")? {
            let player_name = other_player_structured_data.get_string_data("PlayerName")?;
            let player_number = other_player_structured_data.get_u8_data("PlayerNumber")?;
            let player_ip = other_player_structured_data.get_vec_data("PlayerIP")?;
            let player_port = other_player_structured_data.get_u16_data("PlayerPort")?;
            let player_addr_maybe = create_addr_from_ip_and_port(player_ip, player_port);
            player_created_response_data.add_other_player(PlayerData {
                player_name: player_name,
                player_number: player_number,
                addr: match player_addr_maybe {
                    Ok(_) => Some(player_addr_maybe.unwrap()),
                    Err(_) => None,
                },
            })
        }

        // doubel Ok, since using two layers of error handling.
        return Ok(Ok(player_created_response_data));
    } else {
        // something went wrong creating the new player, let's create
        // correct error message for event handler.
        return match status {
            100 => Ok(Err(PlayerCreatedServerError::InvalidRequest)),
            101 => Ok(Err(PlayerCreatedServerError::NameIsTaken)),
            102 => Ok(Err(PlayerCreatedServerError::TooManyPlayers)),
            __ => Ok(Err(PlayerCreatedServerError::InvalidServerStatusCode)),
        };
    }
}
