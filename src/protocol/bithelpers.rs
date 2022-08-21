pub fn get_u8_from_bit_slice(byte: u8, start_bit: usize, bit_count: usize) -> u8 {
    let mut mask: u8 = (!0) << (8 - (start_bit + bit_count));
    mask = mask >> start_bit;

    let move_bits = 8 - (start_bit + bit_count);
    let slice_number = (byte & mask) >> move_bits;

    slice_number
}

pub fn make_f64_from_u8s(bytes: [u8; 8]) -> f64 {
    let num_i = u64::from_be_bytes(bytes);
    f64::from_bits(num_i)
}

pub fn make_u8s_from_f64(number: f64) -> [u8; 8] {
    number.to_be_bytes()
}
