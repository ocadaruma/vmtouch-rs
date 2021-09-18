mod vmtouch;

use crate::vmtouch::MemoryMap;
use env_logger::Env;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "vmtouch")]
struct Options {
    /// Target file
    file: PathBuf,
}

fn main() {
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    let opt = Options::from_args();
    let mmap = MemoryMap::open(opt.file).unwrap();

    println!(
        "Resident pages: {}/{}",
        mmap.resident_pages().unwrap(),
        mmap.pages()
    );
}
