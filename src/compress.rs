use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::io::{BufReader, BufWriter, Result, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::utility;

const DEBUG: bool = utility::DEBUG;
const DEBUG_DICT: bool = utility::DEBUG_DICT;

const VALUES: usize = utility::VALUES;
const ELEM_BYTES: usize = utility::ELEM_BYTES;
const ELEM_HALF: usize = utility::ELEM_HALF;
const NR_ELEMS: usize = utility::ELEMS;
const CHUNK_MAX_SIZE: u64 = utility::CHUNK_MAX_SIZE;
const MIN_OCCATIONS: u64 = utility::MIN_OCCATIONS;

type Reader = utility::Reader;
type Writer = utility::Writer;

struct DictElem {
    data: [u8; ELEM_BYTES],
    occurance: u64,
    useage: u64,
}
impl DictElem {
    pub fn new(arr: [u8; ELEM_BYTES], occ: u64) -> Self {
        DictElem {
            data: arr,
            occurance: occ,
            useage: 0,
        }
    }

    pub fn eq(&self, o: &DictElem) -> bool {
        self.data[0] == o.data[0] && self.data[1] == o.data[1]
    }

    pub fn eq_array(&self, o: &[u8; ELEM_BYTES]) -> bool {
        self.data[0] == o[0] && self.data[1] == o[1]
    }

    pub fn set_occurance(&mut self, occ: u64) {
        self.occurance = occ;
    }

    pub fn increment_useage(&mut self) {
        self.useage += 1;
    }

    pub fn to_string(&self) -> String {
        let t0 = utility::u8_to_string(self.data[0]);
        let t1 = utility::u8_to_string(self.data[1]);

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
                elem_ref.set_occurance(elem.occurance);
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

    pub fn get(&self, index: u8) -> [u8; ELEM_BYTES] {
        self.elems[index as usize].data
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
            out.extend(&self.elems[i].data);
        }

        out
    }

    pub fn increment_useage(&mut self, index: u8) {
        self.elems[index as usize].increment_useage();
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
                let dict_elem = DictElem::new(buf, counter[index] as u64);
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

fn compress_layer(path: &Path, dicts: &mut [(Dictionary, Dictionary)]) -> Result<PathBuf> {
    // creater reader, writer, and buffers
    let mut reader = BufReader::new(File::open(path)?);
    let (path_comp, mut writer) = get_path_and_writer(path)?;

    let mut hits: u64 = 0;
    let mut misses: u64 = 0;
    let mut dict_bytes: u64 = 0;
    let mut overhead: u64 = (dicts.len() * 4) as u64;

    for dict_index in 0..dicts.len() {
        // init dictionary references
        let mut dict_refs = [&mut dicts[dict_index].0, &mut dicts[dict_index].1];

        // dry run
        compress_chunk(true, &mut dict_refs, &mut reader, &mut writer)?;

        //remove unused elements from dictionaries to save extra space
        dict_refs[0].purge_unused();
        dict_refs[1].purge_unused();

        // 1 bytes overhead for each dictionary, and each element uses 2 bytes
        dict_bytes += 2 + 2 * dict_refs[0].len() as u64 + 2 * dict_refs[1].len() as u64;

        // real run
        let (h, m, o) = compress_chunk(false, &mut dict_refs, &mut reader, &mut writer)?;
        hits += h;
        misses += m;
        overhead += o;
    }

    // make sure all buffers are written to file
    writer.flush()?;

    if DEBUG {
        print_comp_result(dicts, path, &path_comp, hits, misses, dict_bytes, overhead)?;
    }

    Ok(path_comp)
}

fn compress_chunk(
    dry: bool,
    dicts: &mut [&mut Dictionary; 2],
    reader: &mut Reader,
    writer: &mut Writer,
) -> Result<(u64, u64, u64)> {
    // init buffers
    let mut rad_buf = [0u8; ELEM_BYTES];
    let mut wri_buf: Vec<u8> = vec![];
    let mut hit_buf: Vec<u8> = vec![];
    let mut mis_buf: Vec<u8> = vec![];

    // init variables
    let mut index: usize = 0;
    let mut hits = 0u64;
    let mut misses = 0u64;
    let mut overhead = 0u64;
    let mut has_read = 0u64;
    let to_read = dicts[0].coverage;

    // get start pos for reader to reset in dry run
    let start_pos = reader.seek(SeekFrom::Current(0))?;

    // start working through the file
    while has_read < to_read {
        // if less remains than can be fed into the read buffer
        if (to_read - has_read) < ELEM_BYTES as u64 {
            if !dry {
                let mut buf_rest = vec![0u8; (to_read - has_read) as usize];
                reader.read_exact(&mut buf_rest)?;
                mis_buf.extend(&buf_rest);
            }

            // we can not read any more bytes from this chunk, so break out of the while loop
            break;
        }

        if let Ok(_) = reader.read_exact(&mut rad_buf) {
            has_read += ELEM_BYTES as u64;

            match dicts[index].get_index(&rad_buf) {
                // matched element in current dict
                Some(elem_index) => {
                    // add element index hits buf
                    hit_buf.push((1 << 7) | elem_index);
                }

                // did not match element in current dict
                None => {
                    let t: (u64, u64);
                    t = manage_hits(dry, &mut wri_buf, &mut hit_buf, &mut mis_buf, dicts[index]);
                    hits += t.0;
                    overhead += t.1;

                    reader.seek(SeekFrom::Current(-(ELEM_HALF as i64)))?;
                    has_read -= ELEM_HALF as u64;
                    mis_buf.extend(&rad_buf[0..ELEM_HALF]);
                    misses += 1;
                    index = if index == 0 { 1 } else { 0 };
                }
            }
        }
    }

    let (h, o) = manage_hits(dry, &mut wri_buf, &mut hit_buf, &mut mis_buf, dicts[index]);
    hits += h;
    overhead += o;

    if !dry {
        write_missed(&mut wri_buf, &mut mis_buf);
        write_to_comp_file(&wri_buf, writer, dicts[0], dicts[1])?;
    } else if dry {
        reader.seek(SeekFrom::Start(start_pos))?;
    }

    Ok((hits, misses, overhead))
}

fn manage_hits(
    dry_run: bool,
    buf_write: &mut Vec<u8>,
    buf_hits: &mut Vec<u8>,
    buf_missed: &mut Vec<u8>,
    dict: &mut Dictionary,
) -> (u64, u64) {
    let mut hits = buf_hits.len() as u64;
    let mut overhead = 0u64;

    // if there are hits to be registered to the write buffer
    if hits > 1 {
        hits += hits;
        if dry_run {
            increment_useages(buf_hits, dict);
        } else {
            overhead += write_missed(buf_write, buf_missed);
            buf_write.extend(&(*buf_hits));
        }

        buf_missed.clear();
        buf_hits.clear();
    }
    // otherwise the hits should be counted as misses instead to minimise overhead
    else if hits > 0 {
        concatinate_hits_to_misses(buf_missed, buf_hits, dict);
        buf_hits.clear();
    }

    (hits, overhead)
}

fn increment_useages(buf_hits: &[u8], dict: &mut Dictionary) {
    for i in 0..buf_hits.len() {
        dict.increment_useage(buf_hits[i] & 0b01111111);
    }
}

fn concatinate_hits_to_misses(buf_missed: &mut Vec<u8>, buf_hits: &[u8], dict: &Dictionary) {
    for i in 0..buf_hits.len() {
        let raw_data = dict.get(buf_hits[i] & 0b01111111);
        buf_missed.extend(&raw_data);
    }
}

fn write_missed(buf_write: &mut Vec<u8>, buf_missed: &[u8]) -> u64 {
    let missed = buf_missed.len();
    let mut overhead = 0;

    // if a lot of raw values needs to be written first
    if missed >= VALUES / 2 {
        let nr_bytes = utility::bytes_to_rep(missed);
        let bytes = utility::val_to_u8_vec(missed, nr_bytes);
        buf_write.push(nr_bytes);
        buf_write.extend(&bytes);
        overhead += (1 + bytes.len()) as u64;
    }
    // if only a few raw values needs to be written first
    else if missed > 0 {
        let miss_byte = ((1 << 6) | missed) as u8;
        buf_write.push(miss_byte);
        overhead += 1;
    }

    buf_write.extend(buf_missed.to_vec());
    overhead
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
    let chunk_len_buf = utility::val_to_u8_vec(len, bytes as u8);

    writer.write_all(&chunk_len_buf)?;

    // add buf_write content to out file
    writer.write_all(&buf_final)?;

    Ok(())
}

fn print_comp_result(
    dictionaries: &[(Dictionary, Dictionary)],
    path: &Path,
    path_comp: &Path,
    hit_data: u64,
    miss_data: u64,
    dict_data: u64,
    overhead_data: u64,
) -> Result<()> {
    println!(
        "\n\nLAYER RESULT: {} -> {} \nCOMPRESSED: {}. NON-COMPRESSED: {}. DICT: {}, OVERHEAD: {}",
        path.metadata()?.len(),
        path_comp.metadata()?.len(),
        hit_data,
        miss_data,
        dict_data,
        overhead_data
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

fn finalize_file(path: &Path, layers: u8) -> Result<PathBuf> {
    let (final_path, mut writer) = get_final_writer(path)?;
    let mut file = File::open(path)?;
    let mut buf: Vec<u8> = vec![layers];

    // load file into buf
    file.read_to_end(&mut buf)?;
    writer.write_all(&buf)?;
    writer.flush()?;

    if DEBUG {
        println!("\nFinal file: {} bytes with {} layers", buf.len(), layers);
    }

    Ok(final_path)
}

fn get_final_writer(path: &Path) -> Result<(PathBuf, BufWriter<File>)> {
    let extension = path.extension().unwrap().to_str().unwrap();
    // if the path has a tmp extension, remove the tmp extension, otherwise keep the file as is
    let stem = if extension.find("tmp") != None {
        path.file_stem().unwrap().to_str().unwrap()
    } else {
        path.to_str().unwrap()
    };

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
