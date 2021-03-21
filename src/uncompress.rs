use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::io::{BufReader, BufWriter, Result, SeekFrom};
use std::path::{Path, PathBuf};

use crate::utility;

const DEBUG: bool = utility::DEBUG;
const DEBUG_DICT: bool = utility::DEBUG_DICT;

const ELEM_BYTES: usize = utility::ELEM_BYTES;

type Reader = utility::Reader;
type Writer = utility::Writer;

struct DictElem {
    data: [u8; ELEM_BYTES],
}

impl DictElem {
    pub fn new(slice: [u8; ELEM_BYTES]) -> Self {
        DictElem { data: slice }
    }

    pub fn to_string(&self) -> String {
        let mut out: String = String::from("( ");
        for i in 0..ELEM_BYTES {
            out.push_str(utility::u8_to_string(self.data[i]).as_str());
            // if not last value add a comma after written val
            if i < ELEM_BYTES - 1 {
                out.push_str(", ");
            }
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

    pub fn get(&self, index: u8) -> [u8; ELEM_BYTES] {
        self.elems[index as usize].data
    }

    pub fn get_dict_elem(&self, index: u8) -> &DictElem {
        &self.elems[index as usize]
    }

    pub fn len(&self) -> usize {
        self.elems.len()
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
    let mut reader = BufReader::new(File::open(path)?);
    let mut old_path = path.to_path_buf();
    let layers = get_layers(&mut reader)?;

    if DEBUG {
        println!("\nUncompressing {} layers\n", layers);
    }

    // if there is no compression, just read file into output to remove added byte in begining
    if layers == 0 {
        let (out, mut writer) = get_path_and_writer(path)?;
        let mut buf = vec![];
        reader.read_to_end(&mut buf)?;
        writer.write_all(&buf)?;
        writer.flush()?;
        old_path = out;
    }

    for layer in 0..layers {
        let new_path = uncompress_layer(&old_path, &mut reader)?;
        reader = BufReader::new(File::open(&new_path)?);

        if DEBUG {
            let old_l = old_path.metadata()?.len();
            let new_l = new_path.metadata()?.len();
            println!("\nUncompressed layer {}. {} -> {}", layer, old_l, new_l);
        }

        if layer > 0 {
            // remove extra files that are finished
            std::fs::remove_file(&old_path)?;
        }

        old_path = new_path;
    }

    let final_path = finalize_file(&old_path)?;
    Ok(final_path)
}

fn get_layers(reader: &mut Reader) -> Result<u8> {
    let mut layer_slice = [0u8];
    reader.read_exact(&mut layer_slice)?;

    Ok(layer_slice[0])
}

fn uncompress_layer(path: &Path, reader: &mut Reader) -> Result<PathBuf> {
    // get curent pos
    let current = reader.seek(SeekFrom::Current(0))?;
    // get layer bytes (if on layer 0 then current is > 0)
    let bytes_in_layer = reader.seek(SeekFrom::End(0))? - current;
    // reset to current pos
    reader.seek(SeekFrom::Start(current))?;

    if DEBUG {
        println!("Layer length: {}", bytes_in_layer);
    }

    // set read bytes to current pos in file
    let mut bytes_read = current;
    let (out, mut writer) = get_path_and_writer(path)?;

    while bytes_read < bytes_in_layer {
        bytes_read += uncompress_chunk(&mut writer, reader)?;
    }

    writer.flush()?;
    Ok(out)
}

fn uncompress_chunk(writer: &mut Writer, reader: &mut Reader) -> Result<u64> {
    let mut dicts: Vec<Dictionary> = Vec::new();
    let mut buf_chunk_total = [0u8; 4];

    reader.read_exact(&mut buf_chunk_total)?;
    let chunk_total = utility::u8_vec_to_u32(&buf_chunk_total) as u64;

    if DEBUG {
        println! {"1: start of chunk with length {}", chunk_total};
    }

    dicts.push(get_dictionary(reader)?);
    dicts.push(get_dictionary(reader)?);
    let dict_bytes = (2 + (2 * dicts[0].len()) + (2 * dicts[1].len())) as u64;
    let chunk = chunk_total - dict_bytes - 4;

    if DEBUG_DICT {
        println!(
            "Dict 1: {}\nDict 2: {}\n\n",
            dicts[0].to_string(),
            dicts[1].to_string()
        );
    }

    let mut buf = [0u8];
    let mut dict_index = 0;
    let mut read = 0;

    while read < chunk {
        if DEBUG {
            println! {"4: read = {} , chunk = {}", read, chunk};
        }

        reader.read_exact(&mut buf)?;
        read += 1;
        let byte = buf[0];
        let hit = ((byte >> 7) & 1) == 1;

        if hit {
            let index = byte & 0b01111111;
            let dict_element = dicts[dict_index].get(index);

            if DEBUG {
                println! {"5: hit! index: {} , dict val: {}", index, dicts[dict_index].get_dict_elem(index).to_string()};
            }

            writer.write_all(&dict_element)?;
        } else {
            let is_short = ((byte >> 6) & 1) == 1;
            let val_part = byte & 0b00111111;

            let miss_bytes: usize = if is_short {
                val_part as usize
            } else {
                let mut buf_miss_bytes = vec![0u8; val_part as usize];

                if DEBUG {
                    println! {"7: to represent value: {}", val_part};
                }

                reader.read_exact(&mut buf_miss_bytes)?;
                read += val_part as u64;
                utility::u8_vec_to_u64(&buf_miss_bytes) as usize
            };

            if miss_bytes % 2 == 1 {
                dict_index = if dict_index == 0 { 1 } else { 0 };
            }

            let mut buf_miss = vec![0u8; miss_bytes];

            if DEBUG {
                println! {"8: missed_bytes: {}", miss_bytes};
            }

            reader.read_exact(&mut buf_miss)?;
            read += miss_bytes as u64;
            writer.write_all(&buf_miss)?;
        }
    }

    Ok(chunk_total)
}

fn get_path_and_writer(path: &Path) -> Result<(PathBuf, BufWriter<File>)> {
    let f_st = path.file_stem().unwrap().to_str().unwrap();
    let f_ex = path.extension().unwrap().to_str().unwrap();
    let is_tmp = f_ex.find("tmp") != None;

    let end_nr = 1 + if is_tmp {
        f_ex.split_at(3).1.parse::<u32>().unwrap()
    } else {
        0
    };

    let path_comp = PathBuf::from(format!("{}.tmp{}", f_st, end_nr));

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

fn get_dictionary(reader: &mut Reader) -> Result<Dictionary> {
    let mut dict = Dictionary::new();
    let mut buf_short = [0u8];
    let mut buf = [0u8; 2];

    reader.read_exact(&mut buf_short)?;
    let len = buf_short[0];
    if DEBUG {
        println! {"2: read dictionary with {} elements", len};
    }

    for _ in 0..len {
        reader.read_exact(&mut buf)?;
        let elem = DictElem::new(buf);
        if DEBUG {
            println! {"3: dict elem = {}", elem.to_string()};
        }
        dict.push(elem);
    }

    Ok(dict)
}

fn finalize_file(path: &Path) -> Result<PathBuf> {
    let path_wo_tmp = Path::new(path.file_stem().unwrap());
    let stem = path_wo_tmp.file_stem().unwrap().to_str().unwrap();
    let extn = path_wo_tmp.extension().unwrap().to_str().unwrap();

    let path_final = if path_wo_tmp.exists() {
        let mut s = String::new();
        s.push_str(stem);
        s.push_str("_decompressed.");
        s.push_str(extn);
        PathBuf::from(s)
    } else {
        PathBuf::from(path_wo_tmp)
    };

    std::fs::rename(path, &path_final)?;
    Ok(path_final)
}
