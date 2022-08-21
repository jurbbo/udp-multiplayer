use crate::protocol::builders::RawDataBuilder;
use crate::protocol::DataStructure;
use crate::protocol::HashMap;
use crate::protocol::Protocol;
use crate::protocol::ProtocolError;
use crate::server::connection::Connection;
use std::io::ErrorKind;

pub fn get_protocol_total_length(structures: &HashMap<String, DataStructure>) -> usize {
    let mut protocol_total_length: usize = 0;
    for (_key, structure) in structures {
        protocol_total_length += structure.length;
    }
    protocol_total_length
}

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

pub fn get_ip_and_port_from_socket_addr(addr: SocketAddr) -> Result<([u8; 4], u16), ErrorKind> {
    let port = addr.port();
    let ip = addr.ip();
    if let IpAddr::V4(ipv4) = ip {
        return Ok((ipv4.octets(), port));
    }

    Err(ErrorKind::InvalidData)
}

pub fn create_addr_from_ip_and_port(ip_vec: Vec<u8>, port: u16) -> Result<SocketAddr, ErrorKind> {
    if ip_vec.len() != 4 {
        return Err(ErrorKind::InvalidData);
    }

    let ip = [ip_vec[0], ip_vec[1], ip_vec[2], ip_vec[3]];
    Ok(SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3])),
        port,
    ))
}

pub fn create_player_request(
    protocols: &Protocol,
    player_name: String,
) -> Result<Vec<u8>, ProtocolError> {
    let player_enter_request_protocol = protocols.get_protocol("PlayerEnterRequest")?;

    let raw_data = RawDataBuilder::new(false)
        .add_string_data("PlayerName", player_enter_request_protocol, player_name)?
        .get_raw_data();

    Ok(raw_data)
}

pub fn create_player_enter_push(
    protocols: &Protocol,
    player_name: String,
    player_number: u8,
    player_addr: SocketAddr,
) -> Result<Vec<u8>, ProtocolError> {
    let player_enter_push_protocol = protocols.get_protocol("PlayerEnterPush")?;

    let player_ip_and_port_maybe = get_ip_and_port_from_socket_addr(player_addr);

    let player_ip = match player_ip_and_port_maybe {
        Ok(_) => player_ip_and_port_maybe.unwrap().0,
        Err(_) => [0, 0, 0, 0],
    };

    let player_port = match player_ip_and_port_maybe {
        Ok(_) => player_ip_and_port_maybe.unwrap().1,
        Err(_) => 0,
    };

    let raw_data = RawDataBuilder::new(false)
        .add_vec_data(
            "PlayerNumber",
            player_enter_push_protocol,
            vec![player_number],
        )?
        .add_string_data("PlayerName", player_enter_push_protocol, player_name)?
        .add_vec_data("PlayerIP", player_enter_push_protocol, player_ip.to_vec())?
        .add_u16_data("PlayerPort", player_enter_push_protocol, player_port)?
        .test_byte_length(player_enter_push_protocol)?
        .get_raw_data();

    Ok(raw_data)
}

/*
"PlayerCreatedResponse" => {
    let structures = DataStructuresFactory::new()
        .structure("Status", 1, DataType::NUMBERDATA, None)?
        .structure("PlayerNumber", 1, DataType::NUMBERDATA, None)?
        .structure("PlayerName", 15, DataType::STRINGDATAFIXEDLENGTH, None)?
        .structure(
            "OtherPlayers",
            0,
            DataType::ARRAY,
            Some(
                DataStructuresFactory::new()
                    .structure("PlayerNumber", 1, DataType::NUMBERDATA, None)?
                    .structure("PlayerName", 15, DataType::STRINGDATAFIXEDLENGTH, None)?
                    .structure("PlayerIP", 4, DataType::NUMBERDATA, None)?
                    .structure("PlayerPort", 2, DataType::NUMBERDATA, None)?
                    .get_structures(),
            ),
        )?
        .get_structures();
    Some(structures)
}
*/

/*
    TODO ---> DATAN PITUUTTA EI TESTATA!!!!! DYNAAMISEN PITUUDEN TSEKKAUS PUUTTUU!
*/
pub fn create_player_created_response(
    protocols: &Protocol,
    status: u8,
    player_name: String,
    player_number: u8,
    connections: &HashMap<SocketAddr, Connection>,
) -> Result<Vec<u8>, ProtocolError> {
    let player_created_protocol = protocols.get_protocol("PlayerCreatedResponse")?;

    let mut response_builder = RawDataBuilder::new(false)
        .add_u8_data("Status", player_created_protocol, status)?
        .add_u8_data("PlayerNumber", player_created_protocol, player_number)?
        .add_string_data("PlayerName", player_created_protocol, player_name)?;

    let array_structure =
        protocols.get_array_structure_as_ref("PlayerCreatedResponse", "OtherPlayers")?;

    for (connection_addr, connection) in connections {
        let player_name = connection.player_name.clone();
        let player_number = connection.player_number.clone();

        let player_ip_and_port_maybe = get_ip_and_port_from_socket_addr(*connection_addr);

        let player_ip = match player_ip_and_port_maybe {
            Ok(_) => player_ip_and_port_maybe.unwrap().0,
            Err(_) => [0, 0, 0, 0],
        };

        let player_port = match player_ip_and_port_maybe {
            Ok(_) => player_ip_and_port_maybe.unwrap().1,
            Err(_) => 0,
        };

        let raw_data = RawDataBuilder::new(true)
            .add_u8_data("PlayerNumber", array_structure, player_number)
            .expect("Array builder failed on player number")
            .add_string_data("PlayerName", array_structure, player_name)
            .expect("Array builder failed on IP")
            .add_vec_data("PlayerIP", array_structure, player_ip.to_vec())
            .expect("Array builder failed on IP")
            .add_u16_data("PlayerPort", array_structure, player_port)
            .expect("Array builder failed on IP")
            .get_raw_data();

        response_builder =
            response_builder.add_array_data("OtherPlayers", player_created_protocol, raw_data)?;
    }

    let raw_data = response_builder.get_raw_data();

    Ok(raw_data)
}
