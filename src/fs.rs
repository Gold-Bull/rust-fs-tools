use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, BufWriter},
    path::{self, PathBuf},
};

use bincode::{Decode, Encode};
use jwalk::{Parallelism, WalkDirGeneric};
use rayon::prelude::*;
use std::os::unix::fs::MetadataExt;

#[derive(Encode, Decode, PartialEq, Debug, Clone)]
struct FsEntry {
    pub name: Box<String>,
    pub owner: u32,
    pub group: u32,
    pub mode: u32,
    pub mtime: i64,
    pub inode: u64,
}

#[derive(Encode, Decode, PartialEq, Debug)]
struct FsEntries {
    pub entries: Vec<FsEntry>,
}

fn walk_dir(
    root_path: PathBuf,
    parallelism: Parallelism,
    follow_links: bool,
    sort: bool,
) -> Vec<FsEntry> {
    WalkDirGeneric::<(bool, bool)>::new(root_path)
        .follow_links(follow_links)
        .parallelism(parallelism)
        .sort(sort)
        .into_iter()
        .par_bridge()
        .map(|entry| {
            if entry.is_ok() {
                let entry_unwrap = entry.unwrap();
                if entry_unwrap.depth != 0 && entry_unwrap.metadata().is_ok() {
                    let metadata = entry_unwrap.metadata().unwrap();
                    let name = Box::new(String::from(
                        entry_unwrap
                            .path()
                            .as_os_str()
                            .to_str()
                            .unwrap()
                            .trim_start_matches("./"),
                    ));
                    return Ok(FsEntry {
                        name,
                        owner: metadata.uid(),
                        group: metadata.gid(),
                        mode: metadata.mode(),
                        mtime: metadata.mtime(),
                        inode: metadata.ino(),
                    });
                }
            }
            Err(())
        })
        .filter(|result| result.is_ok())
        .map(|entry| entry.unwrap())
        .collect()
}

pub(crate) fn generate_state(root_path: PathBuf, parallel: Parallelism, state_file: PathBuf) {
    let value: Vec<FsEntry> = walk_dir(root_path, parallel, false, false);

    let entries = FsEntries { entries: value };
    let mut writer = BufWriter::new(File::create(state_file).unwrap());
    bincode::encode_into_std_write(&entries, &mut writer, bincode::config::standard()).unwrap();
}

pub(crate) fn compare_state(
    root_path: PathBuf,
    parallelism: Parallelism,
    read_state_from: PathBuf,
    write_changes_to: Option<PathBuf>,
) {
    let mut map_data: HashMap<String, FsEntry> = HashMap::<String, FsEntry>::new();
    if path::Path::exists(&read_state_from) {
        let mut reader = BufReader::new(File::open(read_state_from).unwrap());
        let decoded: FsEntries =
            bincode::decode_from_std_read(&mut reader, bincode::config::standard()).unwrap();
        map_data = decoded
            .entries
            .into_iter()
            .map(|entry| {
                let name = *entry.name.clone();
                (name, entry)
            })
            .collect::<HashMap<String, FsEntry>>();
    }

    let value: Vec<FsEntry> = walk_dir(root_path, parallelism, false, false)
        .par_iter()
        .map(|entry| entry.clone())
        .filter(|entry| {
            let entry1 = map_data.get_key_value(entry.name.as_str());
            if entry1.is_some() {
                let entry2 = entry1.unwrap();
                let fsentry = entry2.1;
                if entry.owner != fsentry.owner
                    || entry.group != fsentry.group
                    || entry.mode != fsentry.mode
                    || entry.mtime != fsentry.mtime
                    || entry.inode != fsentry.inode
                {
                    return true;
                }
            } else {
                return true;
            }
            false
        })
        .collect();

    if write_changes_to.is_some() {
        let entries = FsEntries { entries: value };
        let mut writer = BufWriter::new(File::create(write_changes_to.unwrap()).unwrap());
        bincode::encode_into_std_write(&entries, &mut writer, bincode::config::standard()).unwrap();
    }
}
