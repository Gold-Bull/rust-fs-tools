mod args;
mod fs;

use self::args::{Args, SubCommand};
use clap::Parser;
use std::num::NonZeroUsize;

fn main() {
    let args = Args::parse();

    let mut now = chrono::Utc::now();
    println!(
        "Start Time: {}",
        now.format("%Y-%m-%d %I-%M-%S %p").to_string()
    );

    let parallelism = args.parallelism();
    let root_path = args.path.clone();

    rayon::ThreadPoolBuilder::new()
        .num_threads(
            args.threads
                .clone()
                .unwrap_or_else(|| NonZeroUsize::new(1).unwrap())
                .get(),
        )
        .build_global()
        .unwrap();

    match &args.cmd {
        Some(SubCommand::GenerateState { write_state_to }) => {
            fs::generate_state(root_path, parallelism, write_state_to.clone().unwrap());
        }
        Some(SubCommand::Compare {
            read_state_from,
            write_changes_to,
        }) => {
            fs::compare_state(
                root_path,
                parallelism,
                read_state_from.clone().unwrap(),
                write_changes_to.clone(),
            );
        },
        None => {
            println!("There was no subcommand given");
        }
    }

    now = chrono::Utc::now();
    println!(
        "End Time: {}",
        now.format("%Y-%m-%d %I-%M-%S %p").to_string()
    );
}
