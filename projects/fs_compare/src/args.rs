use clap::Parser;
use std::{num::NonZeroUsize, path::PathBuf};
use utils::arg_parsers::{check_if_file_exists, check_if_parent_path_exists};

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
        id = "source filesystem state file",
        long = "state-source",
        short = 's',
        value_parser = check_if_file_exists(),
        help = "",
        long_help = "Path to the source filesystem state file"
    )]
    pub src_state: PathBuf,
    #[arg(
        id = "destination filesystem state file",
        long = "state-destination",
        short = 'd',
        value_parser = check_if_file_exists(),
        help = "",
        long_help = "Path to the destination filesystem state file"
    )]
    pub dst_state: PathBuf,
    #[arg(
        id = "threads",
        long,
        short = 't',
        help = "",
        long_help = "Number of threads to use, defaults to CPU count"
    )]
    pub threads: Option<NonZeroUsize>,
    #[arg(
        id = "write differences to file",
        long = "output",
        short = 'o',
        value_parser = check_if_parent_path_exists(),
        help = "",
        long_help="Path to write the differences between the source and destination filesystem states"
    )]
    pub write_changes_to: PathBuf,
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
