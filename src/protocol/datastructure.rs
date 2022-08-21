use crate::protocol::ProtocolError;
use std::collections::HashMap;

#[derive(Debug, std::cmp::PartialEq)]
pub enum DataType {
    STRINGDATA,
    STRINGDATAFIXEDLENGTH,
    NUMBERDATA,
    ARRAY,
    RAWDATA,
}

#[derive(Debug, std::cmp::PartialEq)]
pub struct DataStructure {
    pub data_type: DataType,
    pub start_byte: usize,
    pub length: usize,
    pub array_structure: Option<HashMap<String, DataStructure>>,
}

#[derive(Debug)]
pub struct RawArrayData<'protocol> {
    structure_name: &'protocol str,
    raw_data: Vec<u8>,
    iterator_index: usize,
    length: usize,
    item_size_in_bytes: usize,
    array_structure: &'protocol HashMap<String, DataStructure>,
}

#[derive(Debug)]
pub struct StructuredData<'protocol> {
    protocol: &'protocol HashMap<String, DataStructure>,
    raw_data: Vec<u8>,
    raw_array_data: Option<RawArrayData<'protocol>>,
}

// Array iterator for array data in Structured data
// get_iterable_array returns instance of RawArrayData for iteration.
impl<'protocol> Iterator for RawArrayData<'protocol> {
    type Item = StructuredArray<'protocol>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut item_start = (self.item_size_in_bytes) * self.iterator_index;
        let item_end = item_start + self.item_size_in_bytes;

        println!("{}", item_start);
        println!("{}", item_end);

        if self.raw_data.len() < item_end {
            return None;
        }

        let new_next = &self.raw_data[item_start..item_end];

        self.iterator_index += 1;

        Some(StructuredArray::new(
            self.array_structure,
            new_next.to_vec(),
            self.structure_name,
        ))
    }
}

pub struct StructuredArray<'protocol> {
    protocol: &'protocol HashMap<String, DataStructure>,
    raw_data: Vec<u8>,
    array_structure_name: &'protocol str,
}

impl<'protocol> StructuredArray<'protocol> {
    pub fn new(
        protocol: &'protocol HashMap<String, DataStructure>,
        raw_data: Vec<u8>,
        array_structure_name: &'protocol str,
    ) -> StructuredArray<'protocol> {
        StructuredArray {
            protocol: protocol,
            raw_data: raw_data,
            array_structure_name: array_structure_name,
        }
    }

    fn get(&self, structure_name: &str) -> Result<&DataStructure, ProtocolError> {
        if !self.protocol.contains_key(&structure_name.to_string()) {
            return Err(ProtocolError::DataStructureNotFound);
        }

        Ok(self.protocol.get(&structure_name.to_string()).unwrap())
    }

    fn get_array_structure(&self) -> Result<&HashMap<String, DataStructure>, ProtocolError> {
        let structure = self.get(self.array_structure_name)?;
        let array_structure_maybe = &structure.array_structure;
        if array_structure_maybe.is_none() {
            return Err(ProtocolError::ArrayStructureEmpty);
        }

        Ok(array_structure_maybe.as_ref().unwrap())
    }

    pub fn get_vec_data(&self, structure_name: &str) -> Result<Vec<u8>, ProtocolError> {
        let array_structure = self.get_array_structure()?;

        if !array_structure.contains_key(&structure_name.to_string()) {
            return Err(ProtocolError::DataStructureNotFound);
        }

        let structure = array_structure.get(&structure_name.to_string()).unwrap();

        let raw_data_ok_length = structure.length + structure.start_byte;

        if self.raw_data.len() < raw_data_ok_length as usize {
            return Err(ProtocolError::InvalidRawData);
        }

        let data =
            self.raw_data[structure.start_byte..structure.start_byte + structure.length].to_vec();
        Ok(data)
    }

    pub fn get_u8_data(&self, structure_name: &str) -> Result<u8, ProtocolError> {
        let data = self.get_vec_data(structure_name)?;
        Ok(data[0])
    }

    pub fn get_u16_data(&self, structure_name: &str) -> Result<u16, ProtocolError> {
        let data = self.get_vec_data(structure_name)?;
        if data.len() != 2 {
            return Err(ProtocolError::DataLengthMismatch);
        }
        let data_as_u16 = ((data[0] as u16) << 8) | data[1] as u16;
        Ok(data_as_u16)
    }

    pub fn get_string_data(&self, structure_name: &str) -> Result<String, ProtocolError> {
        let string_vec = self.get_vec_data(structure_name)?;

        let data_string = String::from_utf8_lossy(&string_vec);

        Ok(data_string.into_owned())
    }
}

impl<'protocol> StructuredData<'protocol> {
    pub fn new(
        protocol: &'protocol HashMap<String, DataStructure>,
        raw_data: Vec<u8>,
    ) -> StructuredData {
        StructuredData {
            protocol: protocol,
            raw_data: raw_data,
            raw_array_data: None,
        }
    }
    fn has_array_type(&self) -> bool {
        for (_key, structure) in self.protocol.iter() {
            match structure.data_type {
                DataType::ARRAY => {
                    return true;
                }
                _ => {}
            }
        }
        false
    }

    fn get_array_key_and_structure(&self) -> Option<(&str, &DataStructure)> {
        for (key, structure) in self.protocol.iter() {
            match structure.data_type {
                DataType::ARRAY => {
                    return Some((key, &structure));
                }
                _ => {}
            }
        }
        None
    }

    pub fn insert_data(&mut self, raw_data: Vec<u8>) -> Result<(), ProtocolError> {
        self.raw_data = raw_data;
        Ok(())
    }

    fn prepare_array_data(&mut self, structure_name: &'protocol str) -> Result<(), ProtocolError> {
        /*
        let array_key_and_structure_maybe = self.get_array_key_and_structure();

        if array_key_and_structure_maybe.is_none() {
            // no error here. just do nothing.
            return Ok(());
        }
        */

        /*let (_key, structure) = array_key_and_structure_maybe.unwrap();*/
        let structure = self.get(structure_name)?;
        let array_structure_maybe = &structure.array_structure;
        let array_structure = array_structure_maybe.as_ref().unwrap();

        if array_structure_maybe.is_none() {
            // This is bad: array is set without array structure, return error.
            return Err(ProtocolError::ArrayStructureEmpty);
        }

        let (item_size_in_bytes, array_data) = self.get_raw_array_data(&structure)?;
        let length = array_data[0].clone();
        //let mut array_data_without_first_byte = array_data.clone();
        //array_data_without_first_byte.remove(0);
        self.raw_array_data = Some(RawArrayData {
            structure_name: structure_name,
            raw_data: array_data,
            length: length as usize,
            item_size_in_bytes: item_size_in_bytes,
            array_structure: self.protocol,
            iterator_index: 0,
        });
        Ok(())
    }

    pub fn add_vec_data(
        &mut self,
        structure_name: &String,
        raw_data: Vec<u8>,
    ) -> Result<(), ProtocolError> {
        if !self.protocol.contains_key(structure_name) {
            return Err(ProtocolError::DataStructureNotFound);
        }
        let structure = self.protocol.get(structure_name).unwrap();

        if self.raw_data.len() != structure.start_byte as usize {
            return Err(ProtocolError::BytesMustAddedOrderly);
        }

        self.raw_data.extend(raw_data);

        Ok(())
    }

    pub fn add_string_data(
        &mut self,
        structure_name: &String,
        data: String,
    ) -> Result<(), ProtocolError> {
        if !self.protocol.contains_key(structure_name) {
            return Err(ProtocolError::DataStructureNotFound);
        }
        let structure = self.protocol.get(structure_name).unwrap();

        match structure.data_type {
            DataType::NUMBERDATA => return Err(ProtocolError::WrongStructureDataType),
            DataType::ARRAY => return Err(ProtocolError::WrongStructureDataType),
            _ => {}
        }

        let data_vec = match structure.data_type {
            DataType::STRINGDATA => data.trim().to_owned().as_bytes().to_vec(),
            DataType::STRINGDATAFIXEDLENGTH => {
                let mut data_fixed_length = vec![0, structure.length as u8];
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
        self.raw_data.extend(data_vec);

        Ok(())
    }

    fn get(&self, structure_name: &str) -> Result<&DataStructure, ProtocolError> {
        if !self.protocol.contains_key(&structure_name.to_string()) {
            return Err(ProtocolError::DataStructureNotFound);
        }

        Ok(self.protocol.get(&structure_name.to_string()).unwrap())
    }

    pub fn raw_count(&self) -> usize {
        self.raw_data.len()
    }

    pub fn get_vec_data(&self, structure_name: &str) -> Result<Vec<u8>, ProtocolError> {
        let structure = self.get(structure_name)?;
        let raw_data_ok_length = structure.length + structure.start_byte;

        if self.raw_data.len() < raw_data_ok_length as usize {
            return Err(ProtocolError::InvalidRawData);
        }

        let data =
            self.raw_data[structure.start_byte..structure.start_byte + structure.length].to_vec();
        Ok(data)
    }

    // If data has more than 1 byte, returns first byte of byte vector.
    pub fn get_u8_data(&self, structure_name: &str) -> Result<u8, ProtocolError> {
        let data = self.get_vec_data(structure_name)?;
        Ok(data[0])
    }
    pub fn get_u16_data(&self, structure_name: &str) -> Result<u16, ProtocolError> {
        let data = self.get_vec_data(structure_name)?;
        if data.len() != 2 {
            return Err(ProtocolError::DataLengthMismatch);
        }
        let data_as_u16 = ((data[0] as u16) << 8) | data[1] as u16;
        Ok(data_as_u16)
    }

    pub fn get_iterable_array(
        &mut self,
        structure_name: &'protocol str,
    ) -> Result<RawArrayData, ProtocolError> {
        if !self.has_array_type() {
            return Err(ProtocolError::DataStructureNotFound);
        }

        self.prepare_array_data(structure_name)?;

        if self.raw_array_data.is_none() {
            return Err(ProtocolError::ArrayStructureEmpty);
        }

        Ok(self.raw_array_data.take().unwrap())
    }

    fn get_raw_array_data(
        &self,
        structure: &DataStructure,
    ) -> Result<(usize, Vec<u8>), ProtocolError> {
        if (self.raw_data.len() - 1) <= structure.start_byte {
            return Err(ProtocolError::InvalidRawData);
        }
        return Ok((
            structure.length,
            self.raw_data[structure.start_byte..].to_vec(),
        ));
    }

    pub fn get_string_data(&self, structure_name: &str) -> Result<String, ProtocolError> {
        let string_vec = self.get_vec_data(structure_name)?;

        let data_string = String::from_utf8_lossy(&string_vec);

        Ok(data_string.into_owned())
    }

    pub fn print_protocol_structures(&self) {
        let mut hash_vec: Vec<(&String, &DataStructure)> = self.protocol.iter().collect();
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
        }
    }

    /*
    pub fn get_u8_from_bit_slice(&self, name: String) -> Option<u8> {
        let slice_maybe = self.bit_slices.get(&name);
        if slice_maybe.is_none() {
            return None;
        }

        let slice = slice_maybe.unwrap();

        let byte_number: usize = (slice.start_bit / 8) as usize;
        let bit_number = slice.start_bit % 8;

        let mut mask: u8 = (!0) << (8 - slice.bit_count);

        mask = mask >> bit_number;

        let byte = self.raw_data[byte_number];

        let move_bits = (8 - (slice.start_bit + slice.bit_count));
        let slice_number = (byte & mask) >> move_bits;

        Some(slice_number)
    }
    */
}
