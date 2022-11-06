#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]
mod config;
mod logging;
mod queue;
mod tunnel;

use std::f32::consts::E;

use clap::{Parser, Subcommand};
use log::error;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[clap(flatten)]
    verbose: clap_verbosity_flag::Verbosity,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Queue mx3 files in `path` to pcss
    Queue {
        #[arg()]
        path: String,
    },
    // Pf {
    //     /// lists test values
    //     #[arg(short, long)]
    //     list: bool,
    // },
}

fn main() {
    let config = config::load_config().expect("Error loading the config");
    let cli = Cli::parse();
    env_logger::Builder::new()
        .filter_level(cli.verbose.log_level_filter())
        .format_timestamp(None)
        .format_target(false)
        .init();

    match &cli.command {
        Commands::Queue { path } => match queue::queue(path, config) {
            Ok(()) => {}
            Err(_) => (),
        }, // Commands::Pf { _ } => (),
    }
}
