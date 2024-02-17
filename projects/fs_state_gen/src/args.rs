use clap::Parser;
use jwalk::Parallelism;
use std::{num::NonZeroUsize, path::PathBuf};
use utils::arg_parsers::{check_if_directory_exists, check_if_parent_path_exists};

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
        id = "directory to generate state from",
        long,
        short = 's',
        default_value = ".",
        value_parser = check_if_directory_exists(),
        help = "",
        long_help = "Path to the directory to generate the state file for"
    )]
    pub path: PathBuf,
    #[arg(
        id = "threads",
        long,
        short = 't',
        help = "",
        long_help = "Number of threads to use, defaults to CPU count"
    )]
    pub threads: Option<NonZeroUsize>,
    #[arg(
        id = "write filesystem state to file",
        long = "output",
        short = 'o',
        value_parser = check_if_parent_path_exists(),
        help = "",
        long_help="Path to write the filesystem state file to"
    )]
    pub write_state_to: PathBuf,
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

    pub fn parallelism(&self) -> Parallelism {
        match self.threads() {
            1 => Parallelism::Serial,
            n => Parallelism::RayonNewPool(n),
        }
    }
}
