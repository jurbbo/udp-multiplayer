use crate::client::datahandlers::check_raw_data_length_integrity;
use crate::client::datahandlers::structs::player::PlayerData;
use crate::protocol::datahelpers::create_addr_from_ip_and_port;
use crate::protocol::datastructure::StructuredData;
use crate::protocol::Protocol;
use crate::protocol::ProtocolError;

pub fn structurize_raw_data(
    protocols: &Protocol,
    raw_data: &[u8],
) -> Result<PlayerData, ProtocolError> {
    let player_enter_push_protocol = protocols.get_protocol("PlayerEnterPush")?;

    check_raw_data_length_integrity(player_enter_push_protocol, raw_data)?;

    let structured_data = StructuredData::new(player_enter_push_protocol, raw_data.to_vec());

    let player_name = structured_data.get_string_data("PlayerName")?;
    let player_number = structured_data.get_u8_data("PlayerNumber")?;
    let player_ip = structured_data.get_vec_data("PlayerIP")?;
    let player_port = structured_data.get_u16_data("PlayerPort")?;
    let player_addr_maybe = create_addr_from_ip_and_port(player_ip, player_port);

    return Ok(PlayerData {
        player_name: player_name,
        player_number: player_number,
        addr: match player_addr_maybe {
            Ok(_) => Some(player_addr_maybe.unwrap()),
            Err(_) => None,
        },
    });
}
