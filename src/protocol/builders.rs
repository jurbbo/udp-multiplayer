use crate::protocol::datahelpers;
use crate::protocol::datastructure::{DataStructure, DataType};
use crate::protocol::ProtocolError;
use std::collections::HashMap;

pub struct RawDataBuilder {
    raw_data: Option<Vec<u8>>,
    array_start_byte: usize,
    is_array: bool,
}

const mask_u16_first: u16 = 0b1111_1111_0000_0000;
const mask_u16_second: u16 = 0b0000_0000_1111_1111;

impl RawDataBuilder {
    pub fn new(is_array: bool) -> RawDataBuilder {
        RawDataBuilder {
            raw_data: Some(vec![]),
            array_start_byte: 0,
            is_array: is_array,
        }
    }

    pub fn add_array_data(
        mut self,
        structure_name_str: &str,
        structures: &HashMap<String, DataStructure>,
        mut raw_data: Vec<u8>,
    ) -> Result<RawDataBuilder, ProtocolError> {
        let structure_name = structure_name_str.to_string();
        if !structures.contains_key(&structure_name) {
            return Err(ProtocolError::DataStructureNotFound);
        }
        let structure = structures.get(&structure_name).unwrap();

        if structure.data_type != DataType::ARRAY {
            return Err(ProtocolError::WrongStructureDataType);
        }

        if raw_data.len() % structure.length != 0 {
            return Err(ProtocolError::ArrayRawDataLengthMismatch);
        }

        //let array_length = raw_data.len() / structure.length;
        //let mut data = vec![array_length as u8];
        //data.append(&mut raw_data);
        self.raw_data.as_mut().unwrap().append(&mut raw_data);
        Ok(self)
    }

    pub fn add_vec_data(
        mut self,
        structure_name: &str,
        structures: &HashMap<String, DataStructure>,
        mut raw_data: Vec<u8>,
    ) -> Result<RawDataBuilder, ProtocolError> {
        if !structures.contains_key(structure_name) {
            return Err(ProtocolError::DataStructureNotFound);
        }
        let structure = structures.get(structure_name).unwrap();

        if structure.start_byte == 0 && self.is_array {
            self.array_start_byte = self.raw_data.as_ref().unwrap().len();
        }

        if self.raw_data.as_ref().unwrap().len()
            != self.array_start_byte + structure.start_byte as usize
        {
            return Err(ProtocolError::BytesMustAddedOrderly);
        }

        if raw_data.len() != structure.length {
            return Err(ProtocolError::VecLengthMustMatchStructureLength);
        }

        self.raw_data.as_mut().unwrap().append(&mut raw_data);

        Ok(self)
    }

    pub fn add_u16_data(
        self,
        structure_name: &str,
        structures: &HashMap<String, DataStructure>,
        data_u16: u16,
    ) -> Result<RawDataBuilder, ProtocolError> {
        let first_byte = ((data_u16 & mask_u16_first) >> 8) as u8;
        let second_byte = (data_u16 & mask_u16_second) as u8;

        Ok(self.add_vec_data(structure_name, structures, vec![first_byte, second_byte])?)
    }

    pub fn add_u8_data(
        self,
        structure_name: &str,
        structures: &HashMap<String, DataStructure>,
        data_u8: u8,
    ) -> Result<RawDataBuilder, ProtocolError> {
        Ok(self.add_vec_data(structure_name, structures, vec![data_u8])?)
    }

    pub fn add_string_data(
        mut self,
        structure_name: &str,
        structures: &HashMap<String, DataStructure>,
        data: String,
    ) -> Result<RawDataBuilder, ProtocolError> {
        if !structures.contains_key(structure_name) {
            return Err(ProtocolError::DataStructureNotFound);
        }
        let structure = structures.get(structure_name).unwrap();

        match structure.data_type {
            DataType::NUMBERDATA => return Err(ProtocolError::WrongStructureDataType),
            DataType::ARRAY => return Err(ProtocolError::WrongStructureDataType),
            _ => {}
        }

        let data_vec = match structure.data_type {
            DataType::STRINGDATA => data.trim().to_owned().as_bytes().to_vec(),
            DataType::STRINGDATAFIXEDLENGTH => {
                let mut data_fixed_length = vec![0; structure.length];
                let mut count = 0;
                for byte in data.trim().to_owned().as_bytes().to_vec() {
                    data_fixed_length[count] = byte;
                    count += 1;
                    if count >= structure.length {
                        break;
                    }
                }
                data_fixed_length
            }
            _ => return Err(ProtocolError::WrongStructureDataType),
        };
        self.raw_data.as_mut().unwrap().extend(data_vec);

        Ok(self)
    }
    pub fn get_raw_data(&mut self) -> Vec<u8> {
        let raw_data_maybe = self.raw_data.take();
        raw_data_maybe.unwrap()
    }
}

pub struct DataStructuresFactory {
    structures: Option<HashMap<String, DataStructure>>,
}

impl DataStructuresFactory {
    pub fn new() -> DataStructuresFactory {
        DataStructuresFactory {
            structures: Some(HashMap::<String, DataStructure>::new()),
        }
    }
    pub fn structure(
        &mut self,
        name_literal: &str,
        length: usize,
        data_type: DataType,
        array_structure: Option<HashMap<String, DataStructure>>,
    ) -> Result<&mut DataStructuresFactory, ProtocolError> {
        let name = name_literal.to_string();

        // Since string data length is dynamic, we will only allow one string data in protocol as last item.
        // Otherwise start_byte calculations should be also dynamic. In this version, these values
        // are static.
        if self.has_dynamic_data() {
            return Err(ProtocolError::DynamicTypeBeLastItem);
        }

        // Let's prevent overwrites -- if same key name is accidentally inserted.
        if self.structures.as_ref().unwrap().contains_key(&name) {
            return Err(ProtocolError::DataStructureNameMustBeUnique);
        }

        // If data type is Array, one array item length is calculated from array structure.
        let length_calc: usize = match data_type {
            DataType::ARRAY => {
                if array_structure.is_none() {
                    return Err(ProtocolError::ArrayStructureEmpty);
                }

                array_structure
                    .as_ref()
                    .unwrap()
                    .iter()
                    .fold(0, |sum, (_key, value)| {
                        let new_sum: usize = sum + value.length;
                        new_sum
                    })
            }
            _ => length,
        };

        let structure = DataStructure {
            data_type: data_type,
            start_byte: self.calculate_start_byte(),
            length: length_calc,
            array_structure: array_structure,
        };

        self.structures.as_mut().unwrap().insert(name, structure);

        Ok(self)
    }

    pub fn get_structures(&mut self) -> HashMap<String, DataStructure> {
        let structures = self.structures.take();
        structures.unwrap()
    }

    fn has_dynamic_data(&self) -> bool {
        for (_key, structure) in self.structures.as_ref().unwrap() {
            match structure.data_type {
                DataType::STRINGDATA => return true,
                DataType::ARRAY => return true,
                _ => {}
            }
        }
        false
    }

    fn calculate_start_byte(&self) -> usize {
        let mut start_byte: usize = 0;
        for (_name, structure) in self.structures.as_ref().unwrap() {
            start_byte += structure.length;
        }
        start_byte
    }
}
