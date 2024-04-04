use std::path::PathBuf;

use bincode::{Decode, Encode};
use jwalk::{Parallelism, WalkDirGeneric};
use rayon::prelude::*;
use std::os::unix::fs::MetadataExt;

#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub struct FsEntry {
    pub name: String,
    pub owner: u32,
    pub group: u32,
    pub mode: u32,
    pub mtime: i64,
    pub inode: u64,
    pub size: u64,
    pub is_dir: bool,
    pub is_file: bool,
    pub is_symlink: bool,
}

#[derive(Encode, Decode, PartialEq, Debug)]
pub struct FsEntries {
    pub entries: Vec<FsEntry>,
}

#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub struct ChangedFsEntry {
    pub name: String,
    pub is_deleted: bool,
    pub is_dir: bool,
    pub is_file: bool,
    pub is_symlink: bool,
}

#[derive(Encode, Decode, PartialEq, Debug)]
pub struct ChangedFsEntries {
    pub entries: Vec<ChangedFsEntry>,
}

pub fn walk_dir(
    root_path: PathBuf,
    parallelism: Parallelism,
    follow_links: bool,
    skip_hidden: bool,
    sort: bool,
    folder_to_ignore: Vec<String>,
) -> Vec<FsEntry> {
    WalkDirGeneric::<(Vec<String>, bool)>::new(root_path)
        .root_read_dir_state(folder_to_ignore.clone())
        .process_read_dir(|_depth, _path, read_dir_state, children| {
            children.retain(|dir_entry_result| {
                dir_entry_result
                    .as_ref()
                    .map(|dir_entry| {
                        let file_name = dir_entry
                            .file_name
                            .clone()
                            .into_string()
                            .unwrap_or(String::new());
                        !read_dir_state.into_iter().any(|s| s == &file_name)
                    })
                    .unwrap_or(false)
            });
        })
        .skip_hidden(skip_hidden)
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
                    let name = String::from(
                        entry_unwrap
                            .path()
                            .as_os_str()
                            .to_str()
                            .unwrap()
                            .trim_start_matches("./"),
                    );
                    return Ok(FsEntry {
                        name,
                        owner: metadata.uid(),
                        group: metadata.gid(),
                        mode: metadata.mode(),
                        mtime: metadata.mtime(),
                        inode: metadata.ino(),
                        size: metadata.size(),
                        is_dir: metadata.is_dir(),
                        is_file: metadata.is_file(),
                        is_symlink: metadata.is_symlink(),
                    });
                }
            }
            Err(())
        })
        .filter(|result| result.is_ok())
        .map(|entry| entry.unwrap())
        .collect()
}
