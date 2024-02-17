pub(crate) mod args;

use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, BufWriter},
};

use args::Args;
use clap::Parser;
use rayon::{
    iter::{IntoParallelRefIterator, ParallelIterator},
    ThreadPoolBuilder,
};
use utils::fs::{ChangedFsEntries, ChangedFsEntry, FsEntries, FsEntry};

fn main() {
    let args = Args::parse();

    let src_state = args.src_state.clone();
    let dst_state = args.dst_state.clone();
    let write_changes_to = args.write_changes_to.clone();

    ThreadPoolBuilder::new()
        .num_threads(args.threads())
        .build_global()
        .unwrap();

    let src_map_data: HashMap<String, FsEntry>;
    {
        let mut reader = BufReader::new(File::open(src_state).unwrap());
        let decoded: FsEntries =
            bincode::decode_from_std_read(&mut reader, bincode::config::standard()).unwrap();
        src_map_data = decoded
            .entries
            .into_iter()
            .map(|entry| (entry.name.clone(), entry))
            .collect::<HashMap<String, FsEntry>>();
    }

    let dst_map_data: HashMap<String, FsEntry>;
    {
        let mut reader = BufReader::new(File::open(dst_state).unwrap());
        let decoded: FsEntries =
            bincode::decode_from_std_read(&mut reader, bincode::config::standard()).unwrap();
        dst_map_data = decoded
            .entries
            .into_iter()
            .map(|entry| (entry.name.clone(), entry))
            .collect::<HashMap<String, FsEntry>>();
    }

    let mut changed_fs_entries: Vec<ChangedFsEntry> = Vec::new();

    let value: Vec<ChangedFsEntry> = dst_map_data
        .par_iter()
        .map(|(name, fsentry)| {
            let entry_name = name.clone();
            let entry1 = src_map_data.get_key_value(entry_name.as_str());
            if entry1.is_some() {
                let entry2 = entry1.unwrap();
                let src_fsentry = entry2.1;
                if fsentry.owner != src_fsentry.owner
                    || fsentry.group != src_fsentry.group
                    || fsentry.mode != src_fsentry.mode
                    || fsentry.mtime != src_fsentry.mtime
                    || fsentry.inode != src_fsentry.inode
                    || fsentry.size != src_fsentry.size
                    || fsentry.is_dir != src_fsentry.is_dir
                    || fsentry.is_symlink != src_fsentry.is_symlink
                    || fsentry.is_file != src_fsentry.is_file
                {
                    return Ok(ChangedFsEntry {
                        name: entry_name,
                        is_deleted: false,
                        is_dir: fsentry.is_dir,
                        is_file: fsentry.is_file,
                        is_symlink: fsentry.is_symlink,
                    });
                }
            } else {
                return Ok(ChangedFsEntry {
                    name: entry_name,
                    is_deleted: true,
                    is_dir: fsentry.is_dir,
                    is_file: fsentry.is_file,
                    is_symlink: fsentry.is_symlink,
                });
            }
            Err(())
        })
        .filter(|entry| entry.is_ok())
        .map(|entry| entry.unwrap())
        .collect();

    let value1: Vec<ChangedFsEntry> = src_map_data
        .par_iter()
        .map(|(name, fsentry)| {
            let entry_name = name.clone();
            let entry1 = dst_map_data.get_key_value(entry_name.as_str());
            if !entry1.is_some() {
                return Ok(ChangedFsEntry {
                    name: entry_name,
                    is_deleted: false,
                    is_dir: fsentry.is_dir,
                    is_file: fsentry.is_file,
                    is_symlink: fsentry.is_symlink,
                });
            }
            Err(())
        })
        .filter(|entry| entry.is_ok())
        .map(|entry| entry.unwrap())
        .collect();

    changed_fs_entries.extend(value);
    changed_fs_entries.extend(value1);
    changed_fs_entries.dedup_by(|a, b| a.name.eq_ignore_ascii_case(&b.name));

    let entries = ChangedFsEntries {
        entries: changed_fs_entries,
    };
    {
        let mut writer = BufWriter::new(File::create(write_changes_to).unwrap());
        bincode::encode_into_std_write(&entries, &mut writer, bincode::config::standard()).unwrap();
    }
}
