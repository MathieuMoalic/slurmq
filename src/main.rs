#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]
mod config;
mod logging;
mod queue;
mod tunnel;

use std::path::Path;

use clap::{Parser, Subcommand};
use log::{debug, error};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[clap(flatten)]
    verbose: clap_verbosity_flag::Verbosity,

    #[command(subcommand)]
    command: Commands,
}

const SSH_CONFIG_PATH: &str = "~/.ssh/config";
const SBATCH1_PATH: &str = "~/sbatch/amumax_fast.sh";
const REMOTE_JOB_DIR: &str = "jobs";

#[derive(Subcommand)]
enum Commands {
    /// Queue mx3 files in `path` to pcss
    Queue {
        #[arg()]
        host: String,
        #[arg()]
        path: String,
    },
}

fn main() {
    let cli = Cli::parse();
    env_logger::Builder::new()
        .filter_level(cli.verbose.log_level_filter())
        .format_timestamp(None)
        .format_target(false)
        .init();

    match &cli.command {
        Commands::Queue { path, host } => queue::main(path, host),
    }
}
