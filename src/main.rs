use std::io::Result;
use std::path::PathBuf;

extern crate clap;
use clap::{App, Arg};

mod compress;
mod uncompress;
mod utility;

#[derive(PartialEq)]
enum Action {
    None,
    Compress,
    Decompress,
}

fn main() -> Result<()> {
    let (path, action) = argument_handler()?;

    let result_path = match action {
        Action::Compress => compress::run(&path)?,
        Action::Decompress => uncompress::run(&path)?,
        Action::None => PathBuf::from(""),
    };

    if action != Action::None {println!("Output: {}", result_path.to_str().unwrap());}
    else {println!("ERROR");}

    Ok(())
}

fn argument_handler() -> Result<(PathBuf, Action)> {
    let matches = App::new("Pyramid Compression")
        .version("1.0")
        .author("Tom Axblad <tom.axblad@gmail.com>")
        .about("A parallel compression algorithm")
        .arg(
            Arg::with_name("compress")
                .short("c")
                .long("compress")
                .required_unless("decompress")
                .conflicts_with("decompress")
                .value_name("COMPRESS_FILE")
                .help("Sets the file to compress")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("decompress")
                .short("d")
                .long("decompress")
                .required_unless("compress")
                .conflicts_with("compress")
                .value_name("DECOMPRESS_FILE")
                .help("Sets the file to decompress")
                .takes_value(true),
        )
        .get_matches();

    let mut action = Action::None;
    let mut path_str = "";

    if let Some(pstr) = matches.value_of("compress") {
        path_str = pstr;
        action = Action::Compress;
    } else if let Some(pstr) = matches.value_of("decompress") {
        path_str = pstr;
        action = Action::Decompress;
    }

    Ok((PathBuf::from(path_str), action))
}
