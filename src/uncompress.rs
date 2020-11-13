use std::io::Result;
use std::path::{Path, PathBuf};

pub fn run(path: &Path) -> Result<PathBuf> {
    println!("Uncompress file {:?}", path.file_name().unwrap());
    Ok(PathBuf::from("result.lc"))
}
