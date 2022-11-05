#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]
use clap::{Parser, Subcommand};
use ssh2::Session;
use std::{
    fs::{self, read_to_string},
    io::Write,
    net::TcpStream,
    path::{Path, PathBuf},
};
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

fn queue(src_dir: &String) {
    let src_dir = Path::new(src_dir);
    let dst_dir = src_dir.strip_prefix(src_dir.parent().unwrap()).unwrap();
    let src_mx3 = find_mx3(src_dir).unwrap();
    // let sess = create_ssh_connection();
    // transfer_files(sess, mx3_paths, src_dir)
}

fn transfer_files(sess: Session, mx3_paths: Vec<PathBuf>, src_dir: &Path) {
    let sftp = sess.sftp().unwrap();
    let stats = sftp.readdir(Path::new(".")).unwrap();
    dbg!(stats);
    // sftp.mkdir(Path::new("hi"), 0o775).unwrap();
    // for src in mx3_paths {
    //     let dest = src.strip_prefix(input_path).unwrap();
    //     println!("{}", dest.display());
    //     println!("{}", dest.parent().unwrap().display());
    //     // let src_buf = fs::read(&src).unwrap();
    //     // let mut dest_buf = sftp.create(dest).expect("Can't create this");
    //     // dest_buf.write_all(&src_buf).unwrap();
    // }
}

fn create_ssh_connection() -> Session {
    let addr = "109.173.138.170:23232";
    let private_key = Path::new("/home/mat/.ssh/id_ed25519");
    let mut sess = Session::new().unwrap();

    let tcp = TcpStream::connect(addr);
    match tcp {
        Ok(tcp) => {
            sess.set_tcp_stream(tcp);
            sess.handshake().unwrap();
            let connection = sess.userauth_pubkey_file("mat", None, private_key, None);
            match connection {
                Ok(()) => (),
                Err(err) => println!("Failed connection: {}", err),
            }
        }
        Err(_) => {}
    };
    sess
}

fn find_mx3(src_dir: &Path) -> Option<Vec<PathBuf>> {
    let mut mx3_paths: Vec<PathBuf> = vec![];
    if src_dir.exists() {
        for entry in WalkDir::new(src_dir)
            .max_depth(1)
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
        println!("Error: Invalid path: {}", src_dir.display());
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
