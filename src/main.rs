use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
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
    Pf {
        /// lists test values
        #[arg(short, long)]
        list: bool,
    },
}

fn queue(path: &String) {
    match find_mx3(path) {
        Some(mx3_paths) => println!("{:?}", mx3_paths),
        _ => (),
    }
}

fn find_mx3(path_string: &String) -> Option<Vec<PathBuf>> {
    let mut mx3_paths: Vec<PathBuf> = vec![];
    let path = Path::new(path_string);
    if Path::new(path).exists() {
        for entry in WalkDir::new(path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let p = entry.into_path();
            if p.is_file() {
                let extension = p.extension().unwrap_or_default();
                if extension == "mx3" {
                    mx3_paths.push(p);
                }
            }
        }
    } else {
        println!("Error: Invalid path: {}", path_string);
        return None;
    }
    Some(mx3_paths)
}

fn main() {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Queue { path } => {
            queue(path);
        }
        Commands::Pf { list } => {
            if *list {
                println!("Printing testing lists...");
            } else {
                println!("Not printing testing lists...");
            }
        }
    }
}
