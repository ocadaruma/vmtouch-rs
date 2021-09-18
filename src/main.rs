mod vmtouch;

use crate::vmtouch::MemoryMap;
use env_logger::Env;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(name = "vmtouch")]
struct Options {
    /// Target file
    #[structopt(short, long)]
    file: PathBuf,

    #[structopt(subcommand)]
    cmd: Option<Command>,
}

#[derive(StructOpt)]
enum Command {
    Evict,
    Touch,
}

fn main() {
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    let opt = Options::from_args();
    let mut mmap = MemoryMap::open(opt.file).unwrap();

    match opt.cmd {
        Some(cmd) => match cmd {
            Command::Evict => mmap.evict().unwrap(),
            Command::Touch => mmap.touch(),
        },
        _ => {
            println!(
                "Resident pages: {}/{}",
                mmap.resident_pages(),
                mmap.pages()
            );
        }
    }
}
