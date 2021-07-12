use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::io::{BufReader, BufWriter, Result, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::comp_structs::{dict_elem::DictElem, dictionary::Dictionary};
use crate::utility;
use crate::utility::{
    Reader, Writer, CHUNK_MAX_SIZE, DEBUG, DETAILED_DEBUG, DEBUG_DICT, ELEM_BYTES, ELEM_HALF, NR_ELEMS, VALUES_HALF,
};

pub fn run(path: &PathBuf) -> Result<PathBuf> {
    println!("\nCompressing file: {}", path.file_name().unwrap().to_str().unwrap());

    let mut old_path = path.to_owned();
    let mut new_path = path.to_owned();
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
        //continue_compress = false;
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

    for chunk in 0..chunks {
        let offset = chunk * CHUNK_MAX_SIZE;
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
                dict.consider(&dict_elem);
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
    let mut res_buf: Vec<u8> = vec![];

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
        let left_to_read: usize = (to_read - has_read) as usize;

        // if less remains than can be fed into the read buffer
        if left_to_read < ELEM_BYTES {
            if !dry {
                res_buf = vec![0u8; left_to_read];
                reader.read_exact(&mut res_buf)?;
            }

            // we can not read any more bytes from this chunk, so break out of the while loop
            break;
        }

        reader.read_exact(&mut rad_buf)?;
        has_read += ELEM_BYTES as u64;

        match dicts[index].get_index(&rad_buf) {
            // matched element in current dict
            Some(elem_index) => {
                // add element index hits buf
                hit_buf.push((1 << 7) | elem_index);
            }

            // did not match element in current dict
            None => {
                let (h, m, o) = manage_hits(dry, &mut wri_buf, &mut hit_buf, &mut mis_buf, dicts[index]);
                hits += h;
                misses += m;
                overhead += o;

                if !dry && DETAILED_DEBUG && h > 0 {
                    println!("Writing {} bytes from dictionary {}", h / 2, index);
                }

                reader.seek(SeekFrom::Current(-(ELEM_HALF as i64)))?;
                has_read -= ELEM_HALF as u64;
                mis_buf.extend(&rad_buf[0..ELEM_HALF]);
                misses += ELEM_HALF as u64;
                index = if index == 0 { 1 } else { 0 };
            }
        }
    }

    let (h, m, o) = manage_hits(dry, &mut wri_buf, &mut hit_buf, &mut mis_buf, dicts[index]);
    hits += h;
    misses += m;
    overhead += o;

    if !dry && DETAILED_DEBUG && h > 0 {
        println!("Writing {} bytes from dictionary: {}", h / 2, index);
    }

    if !dry {
        mis_buf.extend(&res_buf); // add any elements in end of the chunk to buffered misses
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
) -> (u64, u64, u64) {
    let hits_len = buf_hits.len() as u64;

    let mut hits = 0u64;
    let mut misses = 0u64;
    let mut overhead = 0u64;

    // if there are hits to be registered to the write buffer
    if hits_len > 1 {
        hits = hits_len * 2;
        if dry_run {
            increment_useages(buf_hits, dict);
        } else {
            overhead = write_missed(buf_write, buf_missed);
            buf_write.extend(buf_hits.as_slice());
        }

        buf_missed.clear();
        buf_hits.clear();
    }

    // otherwise the hits should be counted as misses instead to minimise overhead
    else if hits_len > 0 {
        misses = hits_len * 2;
        concatinate_hits_to_misses(buf_missed, buf_hits, dict);
        buf_hits.clear();
    }

    (hits, misses, overhead)
}

fn increment_useages(buf_hits: &[u8], dict: &mut Dictionary) {
    for i in 0..buf_hits.len() {
        dict.increment_usage(buf_hits[i] & 0b01111111);
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
    if missed >= VALUES_HALF {
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

    if DETAILED_DEBUG {
        println!("Writing missed {} Byte(s) {:?}", buf_missed.len(), buf_missed);
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
    buf_final.extend(dict_odd.to_vec());



    // move buf_write data to buf_final
    buf_final.extend(buf_write);

    // add buf_write length to out file as 4 bytes
    let bytes = 4;
    let len = bytes + buf_final.len();
    let chunk_len_buf = utility::val_to_u8_vec(len, bytes as u8);

    writer.write_all(&chunk_len_buf)?;

    // add buf_write content to out file
    writer.write_all(&buf_final)?;

    if DETAILED_DEBUG {
        println!("\nWriting chunk of length {} Bytes to file.\nRaw chunk data:", len);

        let data_per_line = 12;
        let mut data_in_line = 0;

        data_in_line = utility::print_chunk_vec(chunk_len_buf, data_per_line, data_in_line);
        utility::print_chunk_vec(buf_final.to_vec(), data_per_line, data_in_line);

        println!("\n");
    }

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
    let size_before = path.metadata()?.len();
    let size_after = path_comp.metadata()?.len();

    if !(size_after < size_before) {
        return Ok(());
    }

    println!("\nLAYER RESULT:\n{} Bytes -> {} Bytes", size_before, size_after);
    println!("COMPRESSED: {} Bytes. NON-COMPRESSED: {} Bytes. DICTIONARIES: {} Bytes, OVERHEAD: {} Bytes",
        hit_data / 2,
        miss_data,
        dict_data,
        overhead_data,
    );

    if DEBUG_DICT {
        for dict in dictionaries {
            println!(
                "Dict 1: {}\nDict 2: {}\n",
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
