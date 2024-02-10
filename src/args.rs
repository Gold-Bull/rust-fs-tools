use clap::error::{self as clap_error};
use clap::{builder::TypedValueParser, Arg, Command, Parser, Subcommand};
use jwalk::Parallelism;
use std::{fs, num::NonZeroUsize, path::PathBuf};

#[derive(Copy, Clone, Debug)]
#[non_exhaustive]
pub struct PathExistsValueParser {}

impl PathExistsValueParser {
    pub fn new() -> Self {
        Self {}
    }
}

impl TypedValueParser for PathExistsValueParser {
    type Value = String;

    fn parse_ref(
        &self,
        _cmd: &Command,
        _arg: Option<&Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        if value.is_empty() {
            let mut error = clap::Error::new(clap_error::ErrorKind::InvalidValue);
            error.insert(
                clap_error::ContextKind::InvalidValue,
                clap_error::ContextValue::String("Empty value provided".to_string()),
            );
            return Err(error);
        }
        let fs_metadata = fs::metadata(value);
        if !fs_metadata.is_ok() {
            let mut error = clap::Error::new(clap_error::ErrorKind::InvalidValue);
            error.insert(
                clap_error::ContextKind::InvalidValue,
                clap_error::ContextValue::String("Provided path does not exists".to_string()),
            );
            return Err(error);
        }
        Ok(String::from(value.to_str().unwrap()))
    }
}

impl Default for PathExistsValueParser {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Subcommand, Debug)]
pub(crate) enum SubCommand {
    GenerateState {
        #[arg(long = "output", short = 'o', value_parser = PathExistsValueParser::new())]
        write_state_to: Option<PathBuf>,
    },
    Compare {
        #[arg(long = "state-file", short = 'i', value_parser = PathExistsValueParser::new())]
        read_state_from: Option<PathBuf>,
        #[arg(long = "write-changes-to", short = 'c')]
        write_changes_to: Option<PathBuf>,
    },
}

#[derive(Parser, Debug)]
#[clap(author = "Sailesh Bellamkonda", version, about)]
pub(crate) struct Args {
    #[arg(long, short = 's', default_value = ".", value_parser = PathExistsValueParser::new())]
    pub path: PathBuf,
    #[arg(long, short = 't')]
    pub threads: Option<NonZeroUsize>,
    #[command(subcommand)]
    pub cmd: Option<SubCommand>,
}

impl Args {
    pub fn threads(&self) -> usize {
        let cpus = num_cpus::get();
        return self
            .threads
            .clone()
            .unwrap_or_else(|| {
                if cfg!(target_vendor = "apple") {
                    NonZeroUsize::new(4).unwrap()
                } else {
                    std::thread::available_parallelism().unwrap_or(NonZeroUsize::new(cpus).unwrap())
                }
            })
            .get();
    }

    pub fn parallelism(&self) -> Parallelism {
        match self.threads() {
            1 => Parallelism::Serial,
            n => Parallelism::RayonNewPool(n),
        }
    }
}
