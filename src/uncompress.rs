use std::io::Result;
use std::path::{Path, PathBuf};

use crate::utility;

const VALUES: usize = utility::VALUES;
const ELEM_BYTES: usize = utility::ELEM_BYTES;
const NR_ELEMS: usize = utility::ELEMS;
const CHUNK_MAX_SIZE: u64 = utility::CHUNK_MAX_SIZE;
const MIN_OCCATIONS: u64 = utility::MIN_OCCATIONS;

struct DictElem {
    data: [u8; ELEM_BYTES],
}

impl DictElem {
    pub fn new(slice: [u8; ELEM_BYTES]) -> Self {
        DictElem { data: slice }
    }

    pub fn eq(&self, o: &DictElem) -> bool {
        for i in 0..ELEM_BYTES {
            if self.data[i] != o.data[i] {
                return false;
            }
        }

        true
    }

    pub fn eq_array(&self, o: &[u8; ELEM_BYTES]) -> bool {
        for i in 0..ELEM_BYTES {
            if self.data[i] != o[i] {
                return false;
            }
        }

        true
    }

    pub fn to_string(&self) -> String {
        let mut out: String = String::from("( ");
        for i in 0..ELEM_BYTES {
            out.push_str(utility::u8_to_string(self.data[i]).as_str());
            out.push_str(", ");
        }
        out.push_str(")");

        out
    }
}

struct Dictionary {
    elems: Vec<DictElem>,
}

impl Dictionary {
    pub fn new() -> Self {
        Dictionary { elems: vec![] }
    }

    pub fn push(&mut self, elem: DictElem) {
        self.elems.push(elem);
    }

    pub fn get_index(&self, input: &[u8; ELEM_BYTES]) -> Option<u8> {
        for i in 0..self.elems.len() {
            if self.elems[i].eq_array(input) {
                return Some(i as u8);
            }
        }

        None
    }

    pub fn len(&self) -> u8 {
        self.elems.len() as u8
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::with_capacity(self.elems.len() * 2);

        for i in 0..self.elems.len() {
            for j in 0..ELEM_BYTES {
                out.push(self.elems[i].data[j]);
            }
        }

        out
    }

    pub fn to_string(&self) -> String {
        let mut out = String::from(format!("Elements: {}", self.elems.len()));

        for i in 0..self.elems.len() {
            out.push_str(format!("\nElem {}: {}", i, self.elems[i].to_string()).as_str());
        }

        out
    }
}

pub fn run(path: &Path) -> Result<PathBuf> {
    println!(
        "Uncompressing file {}",
        path.file_name().unwrap().to_str().unwrap()
    );

    let path_uncomp = uncompress(path)?;

    Ok(path_uncomp)
}

fn uncompress(path: &Path) -> Result<PathBuf> {
    // TODO implement uncompress code
    Ok(PathBuf::from(path))
}
