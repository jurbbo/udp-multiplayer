pub mod bithelpers;
pub mod builders;
pub mod datahelpers;
pub mod datastructure;
use crate::protocol::builders::DataStructuresFactory;
use crate::protocol::datahelpers::get_protocol_total_length;
use crate::protocol::datastructure::DataStructure;
use crate::protocol::datastructure::DataType;
use crate::requests::jobtype::ClientJob;
use crate::requests::jobtype::ServerJob;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};

type DataStructureT = HashMap<String, DataStructure>;

#[derive(Debug)]
pub enum ProtocolError {
    ProtocolNotFound,
    BytesMustAddedOrderly,
    DataStructureNotFound,
    DataStructureNameMustBeUnique,
    DataLengthMismatch,
    ArrayStructureEmpty,
    ArrayRawDataLengthMismatch,
    InvalidRawData,
    WrongStructureDataType,
    DynamicTypeBeLastItem,
    VecLengthMustMatchStructureLength,
}

impl Display for ProtocolError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match *self {
            ProtocolError::ProtocolNotFound => write!(f, "Protocol not found."),
            ProtocolError::BytesMustAddedOrderly => write!(f, "Bytes must be added orderly. Find right structure order printing the structure."),
            ProtocolError::DataStructureNotFound => write!(f, "Protocol structure not found. Try printing protocol structures to found right name."),
            ProtocolError::DataStructureNameMustBeUnique => write!(f, "Data structure name already exists."),
            ProtocolError::DataLengthMismatch => write!(f, "Data length is not valid. Data missing?"),
            ProtocolError::ArrayStructureEmpty => write!(f, "Array structure is empty."),
            ProtocolError::ArrayRawDataLengthMismatch => write!(f, "Array data length is not valid. Data missing?"),
            ProtocolError::InvalidRawData => write!(f, "Raw data is invalid."),
            ProtocolError::WrongStructureDataType => write!(f, "Wrong structure data type. Print structure and check types."),
            ProtocolError::DynamicTypeBeLastItem => write!(f, "Dynamic data must be last."),
            ProtocolError::VecLengthMustMatchStructureLength => write!(f, "Vector length does not match with structure length."),
        }
    }
}

type ProtocolsT = HashMap<String, DataStructureT>;

pub struct Protocol {
    protocols: ProtocolsT,
}

impl Protocol {
    pub fn new() -> Protocol {
        Protocol {
            protocols: get_default_data_protocols().expect("Protocol builder failed"),
        }
    }
    pub fn get_protocol(&self, protocol_name: &str) -> Result<&DataStructureT, ProtocolError> {
        let protocol_maybe = self.protocols.get(&protocol_name.to_string());
        if protocol_maybe.is_none() {
            return Err(ProtocolError::ProtocolNotFound);
        }
        Ok(protocol_maybe.unwrap())
    }

    pub fn get_structures_ref(
        &self,
        protocol_name: &str,
    ) -> Result<&DataStructureT, ProtocolError> {
        let protocol_maybe = self.protocols.get(&protocol_name.to_string());
        if protocol_maybe.is_none() {
            return Err(ProtocolError::ProtocolNotFound);
        }
        let protocol = protocol_maybe.unwrap();
        Ok(protocol)
    }

    pub fn get_array_structure_as_ref(
        &self,
        protocol_name: &str,
        name: &str,
    ) -> Result<&HashMap<String, DataStructure>, ProtocolError> {
        let protocol_maybe = self.protocols.get(&protocol_name.to_string());
        if protocol_maybe.is_none() {
            return Err(ProtocolError::ProtocolNotFound);
        }
        let structure_maybe = protocol_maybe.unwrap().get(&name.to_string());
        if structure_maybe.is_none() {
            return Err(ProtocolError::DataStructureNotFound);
        }
        let structure = structure_maybe.unwrap();
        if structure.array_structure.is_none() {
            return Err(ProtocolError::WrongStructureDataType);
        }
        Ok(&structure.array_structure.as_ref().unwrap())
    }

    pub fn print_protocol_structures(&self, protocol_name: &str) {
        let protocol = self
            .protocols
            .get(&protocol_name.to_string())
            .expect("Check the protocol name!");

        let mut hash_vec: Vec<(&String, &DataStructure)> = protocol.iter().collect();
        hash_vec.sort_by(|a, b| a.1.start_byte.cmp(&b.1.start_byte));

        for (key, structure) in hash_vec {
            print!("Structure name: {} \n", key);
            print!("Start byte: {}\n", structure.start_byte);
            print!("Length in bytes: {}\n", structure.length);
            print!(
                "Data type: {}\n",
                match structure.data_type {
                    DataType::ARRAY => "Array",
                    DataType::NUMBERDATA => "Number",
                    DataType::STRINGDATA => "String, dynamic length, as last structure only",
                    DataType::STRINGDATAFIXEDLENGTH => "String, fix length",
                    DataType::RAWDATA => "Raw data, dynamic",
                }
            );
            match structure.data_type {
                DataType::ARRAY => {
                    let mut hash_vec: Vec<(&String, &DataStructure)> =
                        structure.array_structure.as_ref().unwrap().iter().collect();
                    hash_vec.sort_by(|a, b| a.1.start_byte.cmp(&b.1.start_byte));
                    for (key, structure) in hash_vec {
                        print!("    Array structure's item name: {}\n", key);
                        print!("    Start byte: {}\n", structure.start_byte);
                        print!("    Length in bytes: {}\n", structure.length);
                        print!(
                            "    Data type: {}\n",
                            match structure.data_type {
                                DataType::ARRAY => "Array",
                                DataType::NUMBERDATA => "Number",
                                DataType::STRINGDATA =>
                                    "String, dynamic length, as last structure only",
                                DataType::STRINGDATAFIXEDLENGTH => "String, fixed length",
                                DataType::RAWDATA => "Raw data, dynamic",
                            }
                        );
                        print!("    ---------------\n");
                    }
                }
                _ => {}
            }
            print!("-------------------\n");
            print!("Total length: {} \n", get_protocol_total_length(protocol));
            print!("-------------------\n");
        }
    }
}

/*
"NoClientAction",
"DataPushRequest",
"DataRequest",
"PlayerEnterRequest",
"PlayerLeaveRequest",
"PingRequest",

"NoServerAction",
"DataPush",
"DataPushDoneResponse",
"DataResponse",
"PlayerCreatedResponse",
"PlayerEnterPush",
"PlayerLeaveResponse",
"PlayerLeavePush",
"PongResponse",

*/

fn get_default_data_protocols() -> Result<HashMap<String, DataStructureT>, ProtocolError> {
    let mut protocols = HashMap::<String, DataStructureT>::new();
    for server_job in ServerJob::as_string() {
        let structures: Option<DataStructureT> = match server_job {
            "PlayerCreatedResponse" => {
                let player_created_response_structures = DataStructuresFactory::new()
                    .structure("Index", 1, DataType::NUMBERDATA, None)?
                    .structure("IsSuccess", 1, DataType::NUMBERDATA, None)?
                    .structure("PlayerNumber", 1, DataType::NUMBERDATA, None)?
                    .structure("PlayerName", 15, DataType::STRINGDATAFIXEDLENGTH, None)?
                    .structure(
                        "OtherPlayers",
                        0,
                        DataType::ARRAY,
                        Some(
                            DataStructuresFactory::new()
                                .structure("PlayerNumber", 1, DataType::NUMBERDATA, None)?
                                .structure("PlayerIP", 4, DataType::NUMBERDATA, None)?
                                .structure("PlayerName", 15, DataType::STRINGDATAFIXEDLENGTH, None)?
                                .get_structures(),
                        ),
                    )
                    .expect("Factory failed")
                    .get_structures();
                Some(player_created_response_structures)
            }
            "PlayerEnterPush" => None,
            _ => None,
        };
        if structures.is_some() {
            protocols.insert(server_job.to_string(), structures.unwrap());
        }
    }
    Ok(protocols)
}
