use std::io::{BufReader, BufWriter};
use std::path::Path;

pub const DEBUG: bool = false;
pub const DETAILED_DEBUG: bool = false;
pub const DEBUG_DICT: bool = false;

pub const ELEM_BYTES: usize = 2;
pub const ELEM_HALF: usize = ELEM_BYTES / 2;
pub const ELEM_BITS: u8 = (ELEM_BYTES * 8) as u8;
pub const NR_ELEMS: usize = 1 << ELEM_BITS;
pub const VALUE_BITS: u8 = ((ELEM_HALF * 8) - 1) as u8;
pub const VALUES: usize = 1 << VALUE_BITS;
pub const VALUES_HALF: usize = VALUES / 2;
pub const CHUNK_MAX_SIZE: u64 = 790000;
pub const MIN_OCCATIONS: u64 = 4;

pub type Reader = BufReader<std::fs::File>;
pub type Writer = BufWriter<std::fs::File>;

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
        let shift = byte * 8;
        let val = ((value >> shift) & 0b11111111) as u8;
        u8_vec.push(val);
    }

    u8_vec
}

pub fn u8_vec_to_u32(s: &[u8; 4]) -> u32 {
    let o1 = (s[0] as u32) << (8 * 3);
    let o2 = (s[1] as u32) << (8 * 2);
    let o3 = (s[2] as u32) << 8;
    let o4 = s[3] as u32;

    o1 | o2 | o3 | o4
}

pub fn u8_vec_to_u64(s: &Vec<u8>) -> u64 {
    let mut val: u64 = 0;
    let last = s.len() - 1;

    for i in 0..=last {
        val |= (s[i] as u64) << (8 * (last - i));
    }

    val
}

pub fn file_is_larger(fa: &Path, fb: &Path) -> bool {
    fa.metadata().unwrap().len() > fb.metadata().unwrap().len()
}

pub fn print_chunk_vec(vec: Vec<u8>, per_line: i32, in_line: i32) -> i32 {
    let mut mut_in_line = in_line;

    for byte in vec.iter() {
        print!("[{}]  \t", byte);
        mut_in_line += 1;
        if mut_in_line == per_line {
            println!();
            mut_in_line = 0;
        }
    }

    mut_in_line
}
