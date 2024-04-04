use clap::Parser;
use std::{num::NonZeroUsize, path::PathBuf};
use utils::arg_parsers::{check_if_directory_exists, check_if_file_exists};

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about,
    help_template = "{before-help}{name} {version}

Author: {author}

{about-with-newline}
{usage-heading} {usage}

{all-args}{after-help}
"
)]
pub(crate) struct Args {
    #[arg(
        id = "source path",
        long = "path-source",
        short = 's',
        value_parser = check_if_directory_exists(),
        help = "",
        long_help = "Path to the source directory"
    )]
    pub src_path: PathBuf,
    #[arg(
        id = "destination path",
        long = "path-destination",
        short = 'd',
        value_parser = check_if_directory_exists(),
        help = "",
        long_help = "Path to the destination directory"
    )]
    pub dst_path: PathBuf,
    #[arg(
        id = "state diff file",
        long = "diff",
        value_parser = check_if_file_exists(),
        help = "",
        long_help = "Path to read the differences between the source and destination filesystem states"
    )]
    pub read_diff_from: PathBuf,
    #[arg(
        id = "chunk_size",
        long,
        short = 'c',
        help = "",
        long_help = "Number of files to process in a single rsync command"
    )]
    pub chunk_size: Option<NonZeroUsize>,
    #[arg(
        id = "threads",
        long,
        short = 't',
        help = "",
        long_help = "Number of threads to use, defaults to CPU count"
    )]
    pub threads: Option<NonZeroUsize>,
    #[arg(
        id = "delete destination",
        long = "delete-destination",
        help = "",
        long_help = "Delete destination filesystem entries that are not present in the source filesystem"
    )]
    pub delete_destination: Option<bool>,
    #[arg(
        id = "temporary directory",
        long = "tmp-dir",
        default_value = "/tmp/fs_tools/",
        value_parser = check_if_directory_exists(),
        help = "",
        long_help = "Temporary directory to store intermediate files"
    )]
    pub tmp_dir: PathBuf,
    #[arg(
        id = "rsync arguments",
        long="rsync-args",
        num_args = 1..,
        default_value = "-q -lptgoD -d --numeric-ids --inplace",
        help = "",
        value_delimiter = ' ',
        long_help = "Arguments to pass to rsync command"
    )]
    pub rsync_args: Vec<String>,
}

impl Args {
    pub fn threads(&self) -> usize {
        let cpus = num_cpus::get();
        self.threads
            .clone()
            .unwrap_or_else(|| {
                if cfg!(target_vendor = "apple") {
                    NonZeroUsize::new(cpus).unwrap()
                } else {
                    std::thread::available_parallelism().unwrap_or(NonZeroUsize::new(cpus).unwrap())
                }
            })
            .get()
    }
}
