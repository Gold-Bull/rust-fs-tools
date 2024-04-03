pub(crate) mod args;

use std::{fs::File, io::BufWriter};

use args::Args;
use clap::Parser;
use rayon::{
    iter::{IntoParallelRefIterator, ParallelIterator},
    ThreadPoolBuilder,
};
use utils::fs::{self as utils_fs, FsEntries, FsEntry};

fn main() {
    let args = Args::parse();

    let parallelism = args.parallelism();
    let root_path = args.path.clone();
    let root_path_str = root_path.to_str().unwrap();
    let write_state_to = args.write_state_to.clone();

    ThreadPoolBuilder::new()
        .num_threads(args.threads())
        .build_global()
        .unwrap();

    let value: Vec<FsEntry> = utils_fs::walk_dir(root_path.clone(), parallelism, false, false)
        .par_iter()
        .map(|entry| {
            let mut entry1 = entry.clone();
            entry1.name = entry1
                .name
                .trim_start_matches(root_path_str)
                .trim_start_matches("/")
                .to_string();
            entry1
        })
        .collect();

    let entries = FsEntries { entries: value };
    let mut writer = BufWriter::new(File::create(write_state_to).unwrap());
    bincode::encode_into_std_write(&entries, &mut writer, bincode::config::standard()).unwrap();
}
