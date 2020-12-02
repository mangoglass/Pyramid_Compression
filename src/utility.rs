use std::path::Path;

pub const VALUE_BITS: u8 = 7;
pub const VALUES: usize = 1 << VALUE_BITS;
pub const ELEM_SIZE: usize = 2;
pub const ELEM_BITS: u8 = 16;
pub const NR_ELEMS: usize = 1 << ELEM_BITS;
pub const CHUNK_MAX_SIZE: u64 = 790000;
pub const MIN_OCCATIONS: u64 = 4;

pub fn u8_to_string(val: u8) -> String {
    if val < 0x80 {
        (val as char).to_string()
    } else {
        format!("{}", val)
    }
}

pub fn bytes_to_rep(value: usize) -> u8 {
    (std::mem::size_of::<usize>() - ((value.leading_zeros() / 8) as usize)) as u8
}

pub fn val_to_u8_vec(value: usize, bytes: u8) -> Vec<u8> {
    let mut u8_vec: Vec<u8> = Vec::with_capacity(bytes as usize);
    for byte in (0..bytes).rev() {
        u8_vec.push((value >> (byte * 8)) as u8);
    }

    u8_vec
}

pub fn file_is_larger(fa: &Path, fb: &Path) -> bool {
    fa.metadata().unwrap().len() > fb.metadata().unwrap().len()
}
