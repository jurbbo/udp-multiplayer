use std::collections::HashMap;

pub fn get_u8_from_bit_slice(byte: u8, start_bit: usize, bit_count: usize) -> u8 {
    let mut mask: u8 = (!0) << (8 - (start_bit + bit_count));
    println!("{:#b}", mask);
    mask = mask >> start_bit;
    println!("{:#b}", mask);

    let move_bits = 8 - (start_bit + bit_count);
    let slice_number = (byte & mask) >> move_bits;

    slice_number
}

pub enum DataType {
    STRINGDATA,
    NUMBERDATA,
}

struct DataBitSlice {
    start_bit: u16,
    bit_count: u16,
    data_type: DataType,
    description: String,
}

pub struct DataStructure {
    bit_slices: HashMap<String, DataBitSlice>,
    raw_data: Vec<u8>,
}

impl DataStructure {
    pub fn new(raw_data: Vec<u8>) -> DataStructure {
        DataStructure {
            bit_slices: HashMap::<String, DataBitSlice>::new(),
            raw_data: raw_data,
        }
    }

    pub fn add_structure(
        &mut self,
        name: String,
        start_bit: u16,
        bit_count: u16,
        data_type: DataType,
        description: String,
    ) {
        let bit_slice = DataBitSlice {
            start_bit: start_bit,
            bit_count: bit_count,
            data_type: data_type,
            description: description,
        };
        self.bit_slices.insert(name, bit_slice);
    }

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
}
