// #![allow(unused_imports)]
// #![allow(unused_variables)]
// #![allow(dead_code)]
// #![allow(clippy::all)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
mod queue;
mod ssh;
mod tunnel;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
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
    /// TCP tunnel to interact with running jobs
    Tunnel {},
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let ssh_con = ssh::Ssh::new(&cli.host, &cli.config_path)?;
    match &cli.command {
        Commands::Queue {
            sbatch,
            input_dir,
            dst_dir,
        } => queue::main(&ssh_con, input_dir, sbatch, dst_dir)?,
        Commands::Tunnel {} => tunnel::main(&ssh_con)?,
    };
    Ok(())
}
