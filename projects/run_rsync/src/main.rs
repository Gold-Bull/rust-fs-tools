pub(crate) mod args;

use std::{
    fs::{create_dir_all, remove_dir_all, remove_file, File},
    io::{BufReader, Read},
    num::NonZeroUsize,
    path::PathBuf,
    process::{self, Stdio},
};

use args::Args;
use clap::Parser;
use rayon::{
    iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator},
    slice::ParallelSlice,
    ThreadPoolBuilder,
};
use utils::fs::{ChangedFsEntries, ChangedFsEntry};

fn create_temporary_directories(tmp_dir: &PathBuf) -> Result<(), String> {
    if !tmp_dir.exists() {
        if create_dir_all(tmp_dir).is_err() {
            return Err(format!("Failed to create directory '{}'", tmp_dir.display()).into());
        }
    }
    let parts_dir = tmp_dir.join("parts");
    if !parts_dir.exists() {
        if create_dir_all(&parts_dir).is_err() {
            return Err(format!("Failed to create directory '{}'", parts_dir.display()).into());
        }
    }
    let logs_dir = tmp_dir.join("logs");
    if !logs_dir.exists() {
        if create_dir_all(&logs_dir).is_err() {
            return Err(format!("Failed to create directory '{}'", logs_dir.display()).into());
        }
    }
    Ok(())
}

fn main() {
    let args = Args::parse();

    let src_path = args.src_path.clone();
    let dst_path = args.dst_path.clone();
    let read_diff_from = args.read_diff_from.clone();
    let job_id = uuid::Uuid::new_v4().to_string();
    let tmp_dir = args.tmp_dir.clone().join(&job_id);
    let rsync_args = args.rsync_args.clone();
    let delete_destination = args.delete_destination;

    ThreadPoolBuilder::new()
        .num_threads(args.threads())
        .build_global()
        .unwrap();

    println!("Job ID: {}", job_id);

    let tmpdir_result = create_temporary_directories(&tmp_dir);
    if tmpdir_result.is_err() {
        eprintln!("{}", tmpdir_result.err().unwrap());
        process::exit(1);
    }
    let tmp_parts_dir = tmp_dir.join("parts");
    let tmp_logs_dir = tmp_dir.join("logs");

    let fs_diff: ChangedFsEntries;
    {
        let mut reader = BufReader::new(File::open(read_diff_from).unwrap());
        fs_diff = bincode::decode_from_std_read(&mut reader, bincode::config::standard()).unwrap();
    }

    let chunk_size = args
        .chunk_size
        .clone()
        .unwrap_or_else(|| {
            let threads = args.threads();
            let changed_files_count = fs_diff.entries.len();
            let l_chunk_size = changed_files_count / threads;
            NonZeroUsize::new(l_chunk_size).unwrap()
        })
        .get();

    println!("Chunk size: {}", chunk_size);

    fs_diff
        .entries
        .par_iter()
        .filter(|entry| !entry.is_deleted)
        .collect::<Vec<&ChangedFsEntry>>()
        .par_chunks(chunk_size)
        .enumerate()
        .for_each(|(index, chunk)| {
            let chunk_number = index + 1;
            println!("CHUNK {:>8}: Processing", chunk_number);
            let src_path = src_path.clone();
            let dst_path = dst_path.clone();
            let files_str = chunk
                .iter()
                .map(|entry| entry.name.clone())
                .collect::<Vec<String>>()
                .join("\0");
            let file_path = tmp_parts_dir.join(format!("part_{}.list", chunk_number));
            {
                std::fs::write(&file_path, files_str).unwrap();
            }
            println!(
                "CHUNK {:>8}: Generated part file '{}'",
                chunk_number,
                file_path.display()
            );
            let rsync_stdout_log = tmp_logs_dir.join(format!("rsync_stdout_{}.log", chunk_number));
            let rsync_stderr_log = tmp_logs_dir.join(format!("rsync_stderr_{}.log", chunk_number));
            let rsync_process = std::process::Command::new("rsync")
                .args(rsync_args.clone())
                .arg(format!(
                    "--log-file={}",
                    tmp_logs_dir
                        .join(format!("rsync_{}.log", chunk_number))
                        .display()
                ))
                .arg(format!("--files-from={}", file_path.display()))
                .arg("--from0")
                .arg(src_path)
                .arg(dst_path)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn();
            if rsync_process.is_err() {
                eprintln!(
                    "CHUNK {:>8}: Failed to spawn rsync process. Error : {}",
                    chunk_number,
                    rsync_process.err().unwrap()
                );
                return;
            }
            let mut rsync_process_unwrap = rsync_process.unwrap();
            let rsync_process_wait = rsync_process_unwrap.wait();
            if rsync_process_wait.is_err() {
                eprintln!(
                    "CHUNK {:>8}: Failed to wait for rsync process. Error : {}",
                    chunk_number,
                    rsync_process_wait.err().unwrap()
                );
                return;
            }
            rsync_process_wait.unwrap();
            if rsync_process_unwrap.stdout.is_some() {
                let mut stdout = rsync_process_unwrap.stdout.unwrap();
                let mut stdout_buf: String = String::new();
                if stdout.read_to_string(&mut stdout_buf).is_err() {
                    eprintln!(
                        "CHUNK {:>8}: Failed to read stdout of rsync process",
                        chunk_number
                    );
                }
                if !stdout_buf.is_empty() && std::fs::write(&rsync_stdout_log, stdout_buf).is_err()
                {
                    eprintln!(
                        "CHUNK {:>8}: Failed to write stdout of rsync process to file",
                        chunk_number,
                    );
                }
            }
            if rsync_process_unwrap.stderr.is_some() {
                let mut stderr = rsync_process_unwrap.stderr.unwrap();
                let mut stderr_buf: String = String::new();
                if stderr.read_to_string(&mut stderr_buf).is_err() {
                    eprintln!(
                        "CHUNK {:>8}: Failed to read stderr of rsync process",
                        chunk_number,
                    );
                }
                if !stderr_buf.is_empty() {
                    eprintln!(
                        "CHUNK {:>8}: Some files/attrs were not transferred",
                        chunk_number
                    );
                    if std::fs::write(&rsync_stderr_log, stderr_buf).is_err() {
                        eprintln!(
                            "CHUNK {:>8}: Failed to write stderr of rsync process to file",
                            chunk_number,
                        );
                    }
                }
            }
        });

    if delete_destination.unwrap_or_else(|| false) {
        fs_diff
            .entries
            .par_iter()
            .filter(|entry| entry.is_deleted)
            .for_each(|entry| {
                let dst_path = dst_path.clone();
                let dir_or_file_path = dst_path.join(entry.name.clone());
                if dir_or_file_path.exists() {
                    if entry.is_dir {
                        if remove_dir_all(&dir_or_file_path).is_err() {
                            eprintln!(
                                "Failed to remove directory '{}'",
                                dir_or_file_path.display()
                            );
                        }
                    } else {
                        if remove_file(&dir_or_file_path).is_err() {
                            if entry.is_file {
                                eprintln!("Failed to remove file '{}'", dir_or_file_path.display());
                            } else if entry.is_symlink {
                                eprintln!(
                                    "Failed to remove symlink '{}'",
                                    dir_or_file_path.display()
                                );
                            }
                        }
                    }
                }
            });
    }
}
