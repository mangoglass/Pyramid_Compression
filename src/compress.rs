use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::io::{BufReader, BufWriter, Result, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::utility;

const DEBUG: bool = utility::DEBUG;
const DEBUG_DICT: bool = utility::DEBUG_DICT;

const VALUES: usize = utility::VALUES;
const ELEM_BYTES: usize = utility::ELEM_BYTES;
const NR_ELEMS: usize = utility::ELEMS;
const CHUNK_MAX_SIZE: u64 = utility::CHUNK_MAX_SIZE;
const MIN_OCCATIONS: u64 = utility::MIN_OCCATIONS;

type Reader = utility::Reader;
type Writer = utility::Writer;

struct DictElem {
    tuple: (u8, u8),
    occurance: u64,
    useage: u64,
}
impl DictElem {
    pub fn new(arr: (u8, u8), occ: u64) -> Self {
        DictElem {
            tuple: arr,
            occurance: occ,
            useage: 0,
        }
    }

    pub fn eq(&self, o: &DictElem) -> bool {
        self.tuple.0 == o.tuple.0 && self.tuple.1 == o.tuple.1
    }

    pub fn eq_array(&self, o: &[u8; 2]) -> bool {
        self.tuple.0 == o[0] && self.tuple.1 == o[1]
    }

    pub fn increment_occurance(&mut self) {
        self.occurance += 1;
    }

    pub fn increment_useage(&mut self) {
        self.useage += 1;
    }

    pub fn to_string(&self) -> String {
        let t0 = utility::u8_to_string(self.tuple.0);
        let t1 = utility::u8_to_string(self.tuple.1);

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
                elem_ref.increment_occurance();
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

    pub fn get_index(&self, input: &[u8; ELEM_BYTES]) -> Option<u8> {
        for i in 0..self.elems.len() {
            if self.elems[i].eq_array(input) {
                return Some(i as u8);
            }
        }

        None
    }

    pub fn purge_unused(&mut self) {
        let mut indexes_to_remove: Vec<usize> = vec![];

        for i in 0..self.elems.len() {
            if self.elems[i].useage == 0 {
                indexes_to_remove.push(i);
            }
        }

        for i in (0..indexes_to_remove.len()).rev() {
            self.elems.remove(indexes_to_remove[i]);
        }
    }

    pub fn len(&self) -> u8 {
        self.elems.len() as u8
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::with_capacity(self.elems.len() * 2);

        for i in 0..self.elems.len() {
            out.push(self.elems[i].tuple.0);
            out.push(self.elems[i].tuple.1);
        }

        out
    }

    pub fn increment_useage(&mut self, index: usize) {
        self.elems[index].increment_useage();
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
    println!(
        "\nCompressing file: {}",
        path.file_name().unwrap().to_str().unwrap()
    );

    let mut old_path = PathBuf::from(path);
    let mut new_path = PathBuf::from(path);
    let mut layers = 0;
    let mut continue_compress = true;

    while continue_compress {
        // if the layer is above 1 then remove temporary file
        if layers > 1 {
            std::fs::remove_file(&old_path)?;
        }

        old_path = new_path;
        let mut dict_collection = generate_dict_collection(&old_path)?;
        new_path = compress_layer(&old_path, &mut dict_collection)?;

        continue_compress = utility::file_is_larger(&old_path, &new_path);
        layers += if continue_compress { 1 } else { 0 };
    }

    let final_path = finalize_file(&old_path, layers)?;

    // only remove old file if there is more than one layer
    if layers > 1 {
        std::fs::remove_file(&old_path)?;
    }

    std::fs::remove_file(&new_path)?;

    Ok(final_path)
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
    let mut reader = BufReader::new(File::open(path)?);
    let mut buf = [0u8; ELEM_BYTES];
    let mut counter = [0u32; NR_ELEMS];

    reader.seek(SeekFrom::Start(offset))?;
    let nr_reads = CHUNK_MAX_SIZE / ELEM_BYTES as u64;

    for _ in 0..nr_reads {
        match reader.read_exact(&mut buf) {
            Ok(()) => {
                let index = ((buf[0] as usize) << 8) | (buf[1] as usize);
                counter[index] += 1;
                let dict_elem = DictElem::new((buf[0], buf[1]), counter[index] as u64);
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

fn compress(path: &Path, dicts: &mut [(Dictionary, Dictionary)]) -> Result<PathBuf> {
    // creater reader, writer, and buffers
    let mut reader = BufReader::new(File::open(path)?);
    let (path_comp, mut writer) = get_path_and_writer(path)?;

    let mut hits: u64 = 0;
    let mut misses: u64 = 0;
    let mut dict_bytes: u64 = 0;

    for dict_index in 0..dicts.len() {
        // init dictionary references
        let mut dict_refs = [&mut dicts[dict_index].0, &mut dicts[dict_index].1];

        // dry run
        compress_loop(true, &mut dict_refs, &mut reader, &mut writer)?;
        dict_refs[0].purge_unused();
        dict_refs[1].purge_unused();

        // real run
        let (h, m, d) = compress_loop(false, &mut dict_refs, &mut reader, &mut writer)?;
        hits += h;
        misses += m;
        dict_bytes += d;
    }

    // make sure all buffers are written to file
    writer.flush()?;

    if DEBUG {
        print_comp_result(dicts, path, &path_comp, hits, misses, dict_bytes)?;
    }

    Ok(path_comp)
}

fn compress_loop(
    dry_run: bool,
    dict_refs: &mut [&mut Dictionary; 2],
    reader: &mut Reader,
    writer: &mut Writer,
) -> Result<(u64, u64, u64)> {
    let dict_bytes = 2 + (2 * dict_refs[0].len() + 2 * dict_refs[1].len()) as u64;

    // init buffers
    let mut buf_read = [0u8; ELEM_BYTES];
    let mut buf_write: Vec<u8> = vec![];
    let mut buf_missed: Vec<u8> = vec![];

    // init variables for dictionary
    let mut hits = 0u64;
    let mut misses = 0u64;
    let mut read_bytes = 0u64;
    let mut ref_index: usize = 0;
    let dict_coverage = dict_refs[0].coverage;

    // start working through the file
    while read_bytes < dict_coverage {
        match reader.read_exact(&mut buf_read) {
            Ok(()) => {
                read_bytes += ELEM_BYTES as u64;

                match dict_refs[ref_index].get_index(&buf_read) {
                    // matched element in current dict
                    Some(elem_index) => {
                        if dry_run {
                            // increment usage of index
                            dict_refs[ref_index].increment_useage(elem_index as usize);
                        } else {
                            // add missed bytes to write_buf
                            write_missed(&mut buf_write, &mut buf_missed, &mut misses);
                            // add element index to file
                            buf_write.push((1 << 7) | elem_index);
                            hits += 1;
                        }
                    }

                    // did not match element in current dict
                    None => {
                        reader.seek(SeekFrom::Current(-1))?;
                        ref_index = if ref_index == 0 { 1 } else { 0 };
                        read_bytes -= 1;

                        if !dry_run {
                            buf_missed.push(buf_read[0]);
                            misses += 1;
                        }
                    }
                }
            }

            Err(_e) => {
                // reached end of file
                if !dry_run && buf_write.len() > 0 {
                    write_missed(&mut buf_write, &mut buf_missed, &mut misses);
                    write_to_comp_file(&buf_write, writer, dict_refs[0], dict_refs[1])?;
                } else if dry_run {
                    reader.seek(SeekFrom::Current(-(read_bytes as i64)))?;
                }

                return Ok((hits, misses, dict_bytes));
            }
        }
    }

    if !dry_run && buf_write.len() > 0 {
        write_missed(&mut buf_write, &mut buf_missed, &mut misses);
        write_to_comp_file(&buf_write, writer, dict_refs[0], dict_refs[1])?;
    } else if dry_run {
        reader.seek(SeekFrom::Current(-(read_bytes as i64)))?;
    }

    Ok((hits, misses, dict_bytes))
}

fn write_missed(buf_write: &mut Vec<u8>, buf_missed: &mut Vec<u8>, miss_data: &mut u64) {
    let missed = buf_missed.len();

    // if a lot of raw values needs to be written first
    if missed > VALUES / 2 {
        let nr_bytes = utility::bytes_to_rep(missed);
        let bytes = utility::val_to_u8_vec(missed, nr_bytes);
        buf_write.push(nr_bytes);
        buf_write.extend(&bytes);
        *miss_data += (1 + bytes.len()) as u64;

        buf_write.extend(buf_missed.to_vec());
        buf_missed.clear();
    }
    // if only a few raw values needs to be written first
    else if missed > 0 {
        let miss_byte = ((1 << 6) | missed) as u8;
        buf_write.push(miss_byte);
        *miss_data += 1;

        buf_write.extend(buf_missed.to_vec());
        buf_missed.clear();
    }
}

fn get_path_and_writer(path: &Path) -> Result<(PathBuf, BufWriter<File>)> {
    let f_ex = path.extension().unwrap().to_str().unwrap();
    let is_tmp = f_ex.find("tmp") != None;
    let f_st: &str;

    let mut end_nr = 1;

    if is_tmp {
        end_nr += f_ex.split_at(3).1.parse::<u32>().unwrap();
        f_st = path.file_stem().unwrap().to_str().unwrap();
    } else {
        f_st = path.to_str().unwrap();
    }

    let path_comp = PathBuf::from(format!("{}.tmp{}", f_st, end_nr));

    let file = OpenOptions::new()
        .write(true)
        .append(false)
        .read(true)
        .create(true)
        .open(&path_comp)?;

    let writer = BufWriter::new(file);

    Ok((path_comp, writer))
}

fn write_to_comp_file(
    buf_write: &[u8],
    writer: &mut Writer,
    dict_eve: &Dictionary,
    dict_odd: &Dictionary,
) -> Result<()> {
    let mut buf_final: Vec<u8> = vec![];

    // add dictionary pair to file
    buf_final.push(dict_eve.len());
    buf_final.extend(dict_eve.to_vec());
    buf_final.push(dict_odd.len());
    buf_final.extend(&dict_odd.to_vec());

    // move buf_write data to buf_final
    buf_final.extend(buf_write);

    // add buf_write length to out file as 4 bytes
    let bytes = 4;
    let len = bytes + buf_final.len();

    writer.write(&utility::val_to_u8_vec(len, bytes as u8))?;

    // add buf_write content to out file
    writer.write(&buf_final)?;

    Ok(())
}

fn print_comp_result(
    dictionaries: &[(Dictionary, Dictionary)],
    path: &Path,
    path_comp: &Path,
    hit_data: u64,
    miss_data: u64,
    dict_data: u64,
) -> Result<()> {
    println!(
        "\n\nBEFORE: {}. AFTER: {} \nTOTAL: {}, COMPRESSED: {}. UNCOMPRESSED: {}. DICTIONARY: {}, OVERHEAD: {}",
        path.metadata()?.len(),
        path_comp.metadata()?.len(),
        hit_data + miss_data + dict_data + (dictionaries.len() * 4) as u64,
        hit_data,
        miss_data,
        dict_data,
        dictionaries.len() * 4
    );

    if DEBUG_DICT {
        for dict in dictionaries {
            println!(
                "\nDict 1: {}\n Dict 2: {}\n",
                dict.0.to_string(),
                dict.1.to_string()
            );
        }
    }

    Ok(())
}

fn finalize_file(path: &Path, layers: usize) -> Result<PathBuf> {
    let (final_path, mut writer) = get_final_writer(path)?;
    let mut file = File::open(path)?;
    let mut buf: Vec<u8> = vec![];

    // load file into buf
    file.read_to_end(&mut buf)?;

    writer.write(&utility::val_to_u8_vec(layers, 4))?;
    writer.write(&buf)?;
    writer.flush()?;

    if DEBUG {
        println!(
            "\nFinal file: {} bytes with {} layers",
            4 + buf.len(),
            layers
        );
    }

    Ok(final_path)
}

fn get_final_writer(path: &Path) -> Result<(PathBuf, BufWriter<File>)> {
    let stem = path.file_stem().unwrap().to_str().unwrap();
    let path_final = PathBuf::from(format!("{}.lc", stem));

    // remove compressed file if it already exists
    if path_final.exists() {
        std::fs::remove_file(&path_final)?;
    }

    let file = OpenOptions::new()
        .write(true)
        .append(false)
        .read(true)
        .create(true)
        .open(&path_final)?;

    let writer = BufWriter::new(file);

    Ok((path_final, writer))
}
