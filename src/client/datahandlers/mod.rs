use crate::protocol::datahelpers;
use crate::protocol::datastructure::DataStructure;
use crate::protocol::ProtocolError;
use std::collections::HashMap;
pub mod playercreatedresponse;
pub mod playerenterpush;
pub mod structs;

fn check_raw_data_length_integrity(
    structures: &HashMap<String, DataStructure>,
    raw_data: &[u8],
) -> Result<(), ProtocolError> {
    let protocol_total_length = datahelpers::get_protocol_total_length(structures);
    if protocol_total_length != raw_data.len() {
        return Err(ProtocolError::DataLengthMismatch);
    }

    Ok(())
}
