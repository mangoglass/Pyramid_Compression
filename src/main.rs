use std::io::Error;
use std::io::ErrorKind;
use std::io::Result;
use std::path::Path;

mod compress;
mod uncompress;

enum Command<T> {
    Compress(T),
    Uncompress(T),
}

fn main() -> Result<()> {
    let args = std::env::args().collect();
    let command = argument_handler(&args)?;

    let path = match command {
        Command::Compress(f) => compress::run(f)?,
        Command::Uncompress(f) => uncompress::run(f)?,
    };

    println!("Resulting file: {}", path.to_str().unwrap());
    Ok(())
}

fn argument_handler(args: &Vec<String>) -> Result<Command<&Path>> {
    assert_eq!(3, args.len());
    let f = Path::new(&args[2]);
    if !f.exists() {
        Err(Error::new(ErrorKind::NotFound, "File does not exist"))
    } else if args[1].ne("-c") && args[1].ne("-u") {
        Err(Error::new(
            ErrorKind::InvalidInput,
            "Invalid flag, only \"-c\" or \"-u\" allowed",
        ))
    } else if args[1].eq("-c") {
        Ok(Command::Compress(f))
    } else {
        Ok(Command::Uncompress(f))
    }
}
