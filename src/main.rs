// #![allow(unused_imports)]
// #![allow(unused_variables)]
// #![allow(dead_code)]
// #![allow(clippy::all)]
// #![warn(clippy::restriction)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
// #![warn(clippy::cargo)]
mod config;
mod queue;
mod tunnel;

use clap::{Parser, Subcommand};
use log::{debug, error};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[clap(flatten)]
    verbose: clap_verbosity_flag::Verbosity,

    #[command(subcommand)]
    command: Commands,
    #[arg(long, default_value_t = String::from("~/.ssh/config"))]
    config_path: String,
    #[arg(long, default_value_t = String::from("pcss"))]
    host: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Queue mx3 files in `path` to pcss
    Queue {
        #[arg(short, long)]
        sbatch: String,
        #[arg(short, long)]
        input_dir: String,
        #[arg(long, default_value_t = String::from("./jobs"))]
        dst_dir: String,
    },
}

fn main() {
    let cli = Cli::parse();
    env_logger::Builder::new()
        .filter_level(cli.verbose.log_level_filter())
        .format_timestamp(None)
        .format_target(false)
        .init();
    let config = match config::load(cli.host, cli.config_path) {
        Ok(config) => {
            debug!("SSH config loaded: {:#?}", config);
            config
        }
        Err(_) => {
            error!("Error loading the SSH config");
            return;
        }
    };
    match &cli.command {
        Commands::Queue {
            sbatch,
            input_dir,
            dst_dir,
        } => queue::main(config, sbatch, input_dir, dst_dir),
    }
}
