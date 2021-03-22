use std::io::Result;
use std::path::PathBuf;

extern crate clap;
use clap::{App, Arg};

mod compress;
mod uncompress;
mod utility;

enum Command<T> {
    Compress(T),
    Uncompress(T),
}

fn main() -> Result<()> {
    let command = argument_handler()?;

    let path = match command {
        Command::Compress(f) => compress::run(f)?,
        Command::Uncompress(f) => uncompress::run(f)?,
    };

    println!("Output: {}", path.to_str().unwrap());
    Ok(())
}

fn argument_handler() -> Result<Command<PathBuf>> {
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

    let mut command: Command<PathBuf>;

    if let Some(pathStr) = matches.value_of("compress") {
        command = Command::Compress(PathBuf::from(pathStr));
    } else if let Some(pathStr) = matches.value_of("decompress") {
        command = Command::Uncompress(PathBuf::from(pathStr));
    }

    Ok(command)
}
