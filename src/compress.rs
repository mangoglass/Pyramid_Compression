use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::io::{BufReader, BufWriter, Result, SeekFrom, Write};
use std::path::{Path, PathBuf};

const BITS_IN_PACKET: u8 = 8;
const BITS_IN_TYPES: u8 = 3;
const BITS_IN_VPT: u8 = 4;
const PACKET: usize = 1 << BITS_IN_PACKET;
const TYPES: usize = 1 << BITS_IN_TYPES;
const VALUES_PER_TYPE: usize = 1 << BITS_IN_VPT;
const LARGEST_TYPE: usize = 1 << TYPES;

struct DataFreq {
    buf: Vec<u8>,
    freq: u64,
}
impl DataFreq {
    pub fn new(vec: &Vec<u8>) -> Self {
        DataFreq {
            buf: vec.to_vec(),
            freq: 0,
        }
    }
}

pub fn run(path: &Path) -> Result<PathBuf> {
    let dict = compression_analyzer(path)?;
    //println!("dict = {:?}", dict);
    let out_path = compress(path, &dict)?;
    Ok(out_path)
}

fn compression_analyzer(path: &Path) -> Result<Vec<Vec<Vec<u8>>>> {
    let mut dict: Vec<Vec<Vec<u8>>> = vec![];
    for i in 1..=TYPES {
        let bytes = 1 << i;
        dict.push(frequent_data(path, bytes)?);
    }

    Ok(dict)
}

fn frequent_data(path: &Path, bytes: usize) -> Result<Vec<Vec<u8>>> {
    let sorted_path = sort_data(path, bytes)?;
    let frequent = find_frequent(bytes, &sorted_path)?;
    std::fs::remove_file(sorted_path)?;

    Ok(frequent)
}

fn sort_data(path: &Path, bytes: usize) -> Result<PathBuf> {
    let mut sorted_path = PathBuf::from("/");

    let mut buf = vec![0u8; bytes];
    let mut p1: &Path;
    let mut p2 = path;
    let mut p_str = "tmp2";

    for i in 1..=bytes {
        p_str = if p_str == "tmp2" { "tmp1" } else { "tmp2" };
        p1 = p2;
        p2 = Path::new(p_str);

        let unsorted_file = File::open(p1)?; // open first file
        std::fs::copy(p1, p2)?; // copy first file to second
        let sorted_file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(false)
            .open(p2)?;

        let mut reader = BufReader::new(unsorted_file); // create reader for first file
        let mut writer = BufWriter::new(sorted_file);
        let byte_counter = &mut [0u64; PACKET]; // create counter for file sorting

        loop {
            match reader.read_exact(&mut buf) {
                Ok(()) => {
                    byte_counter[buf[bytes - i] as usize] += 1;
                }
                Err(_e) => {
                    break;
                }
            }
        }

        for j in 1..PACKET {
            byte_counter[j] += byte_counter[j - 1];
        }

        let f_size = p1.metadata()?.len();
        let mut from_end = bytes as u64 + (f_size % bytes as u64);
        reader.seek(SeekFrom::Start(f_size - from_end))?;

        loop {
            match reader.read_exact(&mut buf) {
                Ok(()) => {
                    let byte_count = &mut byte_counter[buf[bytes - i] as usize];
                    *byte_count -= 1;

                    let pos = *byte_count * bytes as u64;
                    writer.seek(std::io::SeekFrom::Start(pos))?;
                    writer.write(&buf)?;

                    from_end += bytes as u64;
                    if from_end > f_size {
                        break;
                    }

                    reader.seek(SeekFrom::Start(f_size - from_end))?;
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }

        if i > 1 {
            std::fs::remove_file(p1)?;
        }
        if i == bytes {
            sorted_path = p2.to_path_buf();
        }
    }

    Ok(sorted_path)
}

fn find_frequent(bytes: usize, path: &PathBuf) -> Result<Vec<Vec<u8>>> {
    let mut freq_list: Vec<DataFreq> = vec![];
    let mut buf = vec![0u8; bytes];
    let mut stored = DataFreq::new(&vec![]);
    let mut reader = BufReader::new(File::open(path)?);
    let mut smallest_freq = (std::u64::MAX, 0usize);

    while freq_list.len() < VALUES_PER_TYPE {
        match reader.read_exact(&mut buf) {
            Ok(()) => {
                if stored.buf != buf {
                    if stored.freq > 1 {
                        if stored.freq < smallest_freq.0 {
                            smallest_freq = (stored.freq, freq_list.len());
                        }
                        freq_list.push(stored); // store old data in list
                    }

                    stored = DataFreq::new(&buf); // store new data in variable
                    stored.freq += 1;
                } else {
                    stored.freq += 1; // increase frequency of stored
                }
            }
            Err(_e) => {
                if stored.freq > 1 {
                    freq_list.push(stored);
                }
                let out = freq_list.iter().map(|x| x.buf.to_vec()).collect();
                return Ok(out);
            }
        }
    }

    loop {
        match reader.read_exact(&mut buf) {
            Ok(()) => {
                if stored.buf != buf {
                    // if sotred frequency is greater than smallest in list
                    if stored.freq > smallest_freq.0 {
                        freq_list[smallest_freq.1] = stored; // store old data in list
                        smallest_freq = get_smallest_freq(&freq_list); // get new smallest frequency
                    }

                    stored = DataFreq::new(&buf); // store new data in variable
                    stored.freq += 1;
                } else {
                    stored.freq += 1; // increase frequency of stored
                }
            }
            Err(_e) => {
                if stored.freq > smallest_freq.0 {
                    freq_list[smallest_freq.1] = stored;
                }
                break; // reached EOF
            }
        }
    }

    let out = freq_list.iter().map(|x| x.buf.to_vec()).collect();
    Ok(out)
}

fn get_smallest_freq(freq_list: &Vec<DataFreq>) -> (u64, usize) {
    let mut out = (std::u64::MAX, 0usize);

    for i in 0..freq_list.len() {
        let freq = freq_list[i].freq;
        if freq < out.0 {
            out = (freq, i);
        }
    }

    out
}

fn compress(path: &Path, dict: &Vec<Vec<Vec<u8>>>) -> Result<PathBuf> {
    // creater reader, writer, and buffers
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let (path_comp, mut writer) = get_comp_writer(path)?;
    let mut buf: Vec<u8> = vec![0u8; LARGEST_TYPE];
    let mut buf_missed: Vec<u8> = vec![];
    let mut missed: u8 = 0;

    // initialize hash maps over values
    let index_maps = init_comp_hashmaps(dict)?;
    // add dictionary to file
    for i in 0..TYPES {
        let len = dict[i].len();
        writer.write(&[len as u8])?;
        for j in 0..len {
            writer.write(&dict[i][j])?;
        }
    }

    let mut hit_data: u64 = 0;
    let mut miss_data: u64 = 0;

    // start working through the file
    loop {
        match reader.read_exact(&mut buf) {
            Ok(()) => {
                let mut buf_cpy = buf.to_vec();
                let mut written = 0;
                let mut start_type = TYPES;

                while written < LARGEST_TYPE {
                    'write_loop: for d_type in (0..start_type).rev() {
                        // from TYPES-1 to 0
                        match index_maps[d_type].get(&buf_cpy) {
                            Some(index) => {
                                if missed > 0 {
                                    missed_write(&mut writer, &mut buf_missed, &mut missed)?;
                                    miss_data += 1;
                                }

                                let shift = BITS_IN_PACKET - 1;
                                let packet =
                                    (1 << shift) | ((d_type as u8) << BITS_IN_VPT) | *index;

                                writer.write(&[packet])?;
                                written += 1 << (d_type + 1);
                                hit_data += (1 << (d_type + 1)) - 1;

                                if written < LARGEST_TYPE {
                                    let (buf_tmp, tmp) = get_buf_for_type(&buf, written);
                                    buf_cpy = buf_tmp;
                                    start_type = tmp;
                                }

                                break 'write_loop;
                            }

                            None => {
                                if d_type > 0 {
                                    buf_cpy = buf_cpy[0..(1 << d_type)].to_vec();
                                } else {
                                    for j in (1..=TYPES).rev() {
                                        let size = (1 << j) as usize;

                                        if (LARGEST_TYPE - written) % size == 0 {
                                            missed += 1;
                                            buf_missed.append(
                                                &mut buf[written..(written + size)].to_vec(),
                                            );
                                            written += size;
                                            miss_data += size as u64;

                                            if missed == (1 << (BITS_IN_PACKET - 1)) {
                                                missed_write(
                                                    &mut writer,
                                                    &mut buf_missed,
                                                    &mut missed,
                                                )?;
                                                miss_data += 1;
                                            }

                                            if written < LARGEST_TYPE {
                                                let (buf_tmp, tmp) =
                                                    get_buf_for_type(&buf, written);
                                                buf_cpy = buf_tmp;
                                                start_type = tmp;
                                            }

                                            break 'write_loop;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            Err(_e) => {
                // reached end of file
                break;
            }
        }
    }

    println!(
        "COMPRESSED DATA: {} UNCOMPRESSED DATA: {}",
        hit_data, miss_data
    );

    // make sure all buffers are written to file
    writer.flush()?;
    Ok(path_comp)
}

fn init_comp_hashmaps(dict: &Vec<Vec<Vec<u8>>>) -> Result<Vec<HashMap<Vec<u8>, u8>>> {
    let mut index_maps: Vec<HashMap<Vec<u8>, u8>> = vec![];
    for i in 0..TYPES {
        let mut index_map: HashMap<Vec<u8>, u8> = HashMap::new();
        for j in 0..dict[i].len() {
            index_map.insert(dict[i][j].to_vec(), j as u8);
        }
        index_maps.push(index_map);
    }

    Ok(index_maps)
}

fn get_comp_writer(path: &Path) -> Result<(PathBuf, BufWriter<File>)> {
    let path_comp = PathBuf::from(format!(
        "{}.lc",
        path.file_stem().unwrap().to_str().unwrap()
    ));

    let file = OpenOptions::new()
        .write(true)
        .append(true)
        .read(true)
        .create(true)
        .open(&path_comp)?;

    let writer = BufWriter::new(file);

    Ok((path_comp, writer))
}

fn missed_write(
    writer: &mut BufWriter<File>,
    buf_missed: &mut Vec<u8>,
    missed: &mut u8,
) -> Result<()> {
    writer.write(&[*missed - 1])?;
    writer.write(buf_missed)?;
    buf_missed.clear();
    *missed = 0;

    Ok(())
}

fn get_buf_for_type(buf: &Vec<u8>, written: usize) -> (Vec<u8>, usize) {
    let mut out = (vec![], 0);

    for i in (1..=TYPES).rev() {
        let size = 1 << i;
        if (LARGEST_TYPE - written) % size == 0 {
            out = (buf[written..(written + size)].to_vec(), i);
            break;
        }
    }

    out
}
