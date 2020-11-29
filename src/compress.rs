use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::io::{BufReader, BufWriter, Result, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::utility;

const VALUE_BITS: u8 = 7;
const VALUES: usize = 1 << VALUE_BITS;
const ELEM_SIZE: usize = 2;
const ELEM_BITS: u8 = 16;
const NR_ELEMS: usize = 1 << ELEM_BITS;
const CHUNK_MAX_SIZE: u64 = 790000;
const MIN_OCCATIONS: u64 = (ELEM_SIZE * 2) as u64;

struct DictElem {
    tuple: (u8, u8),
    occurance: u64,
}
impl DictElem {
    pub fn new(arr: (u8, u8), occ: Option<u64>) -> Self {
        DictElem {
            tuple: arr,
            occurance: occ.unwrap_or(1),
        }
    }

    pub fn eq(&self, o: &DictElem) -> bool {
        self.tuple.0 == o.tuple.0 && self.tuple.1 == o.tuple.1
    }

    pub fn eq_array(&self, o: &[u8; 2]) -> bool {
        self.tuple.0 == o[0] && self.tuple.1 == o[1]
    }

    pub fn increment(&mut self) {
        self.occurance += 1;
    }

    pub fn to_string(&self) -> String {
        let t0 = utility::u8_to_string(self.tuple.0);
        /*String = if self.tuple.0 < 0x80 {
                    (self.tuple.0 as char).to_string()
                } else {
                    format!("{}", self.tuple.0)
                };
        */
        let t1: String = if self.tuple.1 < 0x80 {
            (self.tuple.1 as char).to_string()
        } else {
            format!("{}", self.tuple.1)
        };

        format!(" ( {}, {} ): {} occations", t0, t1, self.occurance)
    }
}

struct Dictionary {
    elems: Vec<DictElem>,
    least: (usize, u64),
    coverage: u64,
}
impl Dictionary {
    pub fn new() -> Self {
        Dictionary {
            elems: vec![],
            least: (0, 1),
            coverage: CHUNK_MAX_SIZE,
        }
    }

    fn push(&mut self, elem: DictElem) {
        self.elems.push(elem);
    }

    fn replace(&mut self, elem: DictElem, index: usize) {
        self.elems[index] = elem;
    }

    fn redefine_least(&mut self) {
        let mut tmp_lest = (0usize, std::u64::MAX);

        for i in 0..self.elems.len() {
            let occ = self.elems[i].occurance;
            if tmp_lest.1 > occ {
                tmp_lest = (i, occ);
            }
        }

        self.least = tmp_lest;
    }

    fn full(&self) -> bool {
        self.elems.len() >= VALUES
    }

    pub fn consider(&mut self, elem: DictElem) {
        for i in 0..self.elems.len() {
            let elem_ref = &mut self.elems[i];

            if elem_ref.eq(&elem) {
                elem_ref.increment();
                if i == self.least.0 {
                    self.redefine_least();
                }

                return;
            }
        }

        if elem.occurance < MIN_OCCATIONS {
            return;
        } else if !self.full() {
            self.push(elem);
        } else if elem.occurance > self.least.1 {
            self.replace(elem, self.least.0);
            self.redefine_least();
        }
    }

    pub fn get_index(&self, input: &[u8; ELEM_SIZE]) -> Option<u8> {
        for i in 0..self.elems.len() {
            if self.elems[i].eq_array(input) {
                return Some(i as u8);
            }
        }

        None
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::with_capacity(self.elems.len() * 2);

        for i in 0..self.elems.len() {
            out.push(self.elems[i].tuple.0);
            out.push(self.elems[i].tuple.1);
        }

        out
    }

    pub fn to_string(&self) -> String {
        let mut out_str = String::from(format!(
            "coverage: {} bytes. Elements: {}",
            self.coverage,
            self.elems.len()
        ));

        for i in 0..self.elems.len() {
            out_str.push_str(format!("\nElem {}: {}", i, self.elems[i].to_string()).as_str());
        }

        out_str
    }
}

pub fn run(path: &Path) -> Result<PathBuf> {
    let dict_collection = generate_dict_collection(path)?;
    //println!("dict = {:?}", dict);
    let out_path = compress(path, &dict_collection)?;
    Ok(out_path)
}

fn generate_dict_collection(path: &Path) -> Result<Vec<(Dictionary, Dictionary)>> {
    let mut dict_collection: Vec<(Dictionary, Dictionary)> = vec![];
    let file_length = path.metadata()?.len();
    let chunks = 1 + (file_length / CHUNK_MAX_SIZE);

    for i in 0..chunks {
        let offset = i * CHUNK_MAX_SIZE;
        let mut pair = generate_dict_pair(path, offset)?;

        let chunk_size = file_length - offset;
        if chunk_size < CHUNK_MAX_SIZE {
            pair.0.coverage = chunk_size;
            pair.1.coverage = chunk_size;
        }

        dict_collection.push(pair);
    }

    Ok(dict_collection)
}

fn generate_dict_pair(path: &Path, offset: u64) -> Result<(Dictionary, Dictionary)> {
    let even_dict = generate_dict(path, offset)?;
    let odd_dict = generate_dict(path, offset + 1)?;

    Ok((even_dict, odd_dict))
}

fn generate_dict(path: &Path, offset: u64) -> Result<Dictionary> {
    let mut dict = Dictionary::new();

    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut buf = [0u8; ELEM_SIZE];
    let mut counter = [0u32; NR_ELEMS];

    reader.seek(SeekFrom::Start(offset))?;
    let nr_reads = CHUNK_MAX_SIZE / ELEM_SIZE as u64;

    for _ in 0..nr_reads {
        match reader.read_exact(&mut buf) {
            Ok(()) => {
                let index = ((buf[0] as usize) << 8) | (buf[1] as usize);
                counter[index] += 1;
                let dict_elem = DictElem::new((buf[0], buf[1]), Some(counter[index] as u64));
                dict.consider(dict_elem);
            }

            Err(_e) => {
                // reached end of file
                break;
            }
        }
    }

    Ok(dict)
}

fn compress(path: &Path, dictionaries: &[(Dictionary, Dictionary)]) -> Result<PathBuf> {
    // creater reader, writer, and buffers
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let (path_comp, mut writer) = get_comp_writer(path)?;
    let mut buf_read = [0u8; ELEM_SIZE];
    let mut buf_write: Vec<u8> = vec![];
    let mut buf_missed: Vec<u8> = vec![];

    let mut hit_data: u64 = 0;
    let mut miss_data: u64 = 0;
    let mut dict_data: u64 = 0;

    'outer_loop: for dict_index in 0..dictionaries.len() {
        let dict_refs = [&dictionaries[dict_index].0, &dictionaries[dict_index].1];
        buf_write.clear();

        // add dictionary pair to file
        let even_len = dict_refs[0].elems.len();
        let odd_len = dict_refs[1].elems.len();

        buf_write.push(even_len as u8);
        buf_write.push(odd_len as u8);
        buf_write.extend(&dict_refs[0].to_vec());
        buf_write.extend(&dict_refs[1].to_vec());
        dict_data += 2 + (even_len + odd_len) as u64;

        // init variables for dictionary
        let mut read_ptr = 0u64;
        let mut ref_index: usize = 0;

        // start working through the file
        let buffer_reads = dict_refs[0].coverage / (ELEM_SIZE as u64);
        for _ in 0..buffer_reads {
            match reader.read_exact(&mut buf_read) {
                Ok(()) => {
                    read_ptr += ELEM_SIZE as u64;

                    match dict_refs[ref_index].get_index(&buf_read) {
                        // matched element in current dict
                        Some(elem_index) => {
                            let missed = buf_missed.len();

                            // if a lot of raw values needs to be written first
                            if missed > VALUES / 2 {
                                let nr_bytes = utility::bytes_to_rep(missed);
                                let bytes = utility::val_to_u8_vec(missed, nr_bytes);
                                //writer.write(&[nr_bytes])?;
                                //writer.write(&bytes)?;
                                buf_write.push(nr_bytes);
                                buf_write.extend(&bytes);
                                miss_data += (1 + bytes.len()) as u64;

                                //writer.write(&buf_missed)?;
                                buf_write.extend(&buf_missed);
                                buf_missed.clear();
                            }
                            // if only a few raw values needs to be written first
                            else if missed > 0 {
                                let miss_byte = ((1 << 6) | missed) as u8;
                                //writer.write(&[miss_byte])?;
                                buf_write.push(miss_byte);
                                miss_data += 1;

                                //writer.write(&buf_missed)?;
                                buf_write.extend(&buf_missed);
                                buf_missed.clear();
                            }

                            // add element index to file
                            //writer.write(&[(1 << 7) | elem_index])?;
                            buf_write.push((1 << 7) | elem_index);
                            hit_data += 1;
                        }

                        // did not match element in current dict
                        None => {
                            buf_missed.push(buf_read[0]);
                            miss_data += 1;
                            read_ptr -= (ELEM_SIZE / 2) as u64;
                            reader.seek(SeekFrom::Start(read_ptr))?;
                            ref_index = if ref_index == 0 { 1 } else { 0 };
                        }
                    }
                }

                Err(_e) => {
                    if buf_write.len() > 0 {
                        // add buf_write length to out file as 8 bytes
                        writer.write(&utility::val_to_u8_vec(
                            buf_write.len(),
                            std::mem::size_of::<u64>() as u8,
                        ))?;

                        // add buf_write content to out file
                        writer.write(&buf_write)?;
                    }

                    // reached end of file
                    break 'outer_loop;
                }
            }
        }

        // add buf_write length to out file as 8 bytes
        writer.write(&utility::val_to_u8_vec(
            buf_write.len(),
            std::mem::size_of::<u64>() as u8,
        ))?;

        // add buf_write content to out file
        writer.write(&buf_write)?;
    }

    // make sure all buffers are written to file
    writer.flush()?;

    println!(
        "BEFORE: {}. AFTER {} \n COMPRESSED DATA: {}. UNCOMPRESSED DATA: {}. DICTIONARY DATA: {}",
        path.metadata()?.len(),
        path_comp.metadata()?.len(),
        hit_data,
        miss_data,
        dict_data
    );

    for dict in dictionaries {
        println!(
            "\nDict 1: {}\n Dict 2: {}\n",
            dict.0.to_string(),
            dict.1.to_string()
        );
    }

    Ok(path_comp)
}

fn get_comp_writer(path: &Path) -> Result<(PathBuf, BufWriter<File>)> {
    let path_comp = PathBuf::from(format!(
        "{}.lc",
        path.file_stem().unwrap().to_str().unwrap()
    ));

    // remove compressed file if it already exists
    if path_comp.exists() {
        std::fs::remove_file(&path_comp)?;
    }

    let file = OpenOptions::new()
        .write(true)
        .append(false)
        .read(true)
        .create(true)
        .open(&path_comp)?;

    let writer = BufWriter::new(file);

    Ok((path_comp, writer))
}
