use crate::{config::Config, logging::dlog};
use log::{debug, error, info};
use ssh2::{Session, Sftp};
use std::{
    fs,
    io::{Read, Write},
    net::TcpStream,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

pub(crate) fn queue(src_dir: &String, config: Config) -> Result<(), ()> {
    let src_dir = Path::new(src_dir);
    info!("Source directory: {}", src_dir.display());
    let src_mx3 = get_src_mx3(src_dir)?;
    info!("Found these mx3 files: {:#?}", src_mx3);
    let dst_dir = get_dst_dir(src_dir);
    let dst_mx3 = get_dst_mx3(&src_mx3, src_dir, &dst_dir)?;
    debug!("List of mx3 destination paths: {:#?}", dst_mx3);
    let sess = create_ssh_connection(config)?;
    let sftp = dlog(
        sess.sftp(),
        "Made SFTP connection",
        "Failed making the SFTP connection",
    )?;
    info!("SSH connection successful");
    create_dst_dir(&sftp, &dst_dir)?;
    info!("Destination directory created");
    transfer_mx3(&sftp, src_mx3, &dst_mx3)?;
    info!("All .mx3 files transfered successfully.");
    start_jobs(&sess, dst_mx3)?;
    info!("All jobs started successfully.");
    Ok(())
}

fn get_src_mx3(src_dir: &Path) -> Result<Vec<PathBuf>, ()> {
    if src_dir.is_dir() {
        let mut mx3_paths: Vec<PathBuf> = vec![];
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
        if mx3_paths.len() == 0 {
            error!("Couldn't find any .mx3 files in {}", src_dir.display());
            return Err(());
        } else {
            return Ok(mx3_paths);
        }
    } else {
        error!("Directory not found : {}", src_dir.display());
        return Err(());
    }
}

fn get_dst_dir(src_dir: &Path) -> PathBuf {
    let dst_dir = src_dir.strip_prefix(src_dir.parent().unwrap()).unwrap();
    let dst_dir = Path::new("./").join(dst_dir);
    dst_dir
}

fn get_dst_mx3(src_mx3: &Vec<PathBuf>, src_dir: &Path, dst_dir: &Path) -> Result<Vec<PathBuf>, ()> {
    let mut dst_mx3 = vec![];
    for src in src_mx3 {
        let dest = src.strip_prefix(src_dir);
        let dest = dlog(
            dest,
            format!("Stripped `{}` from `{}`", src_dir.display(), src.display()).as_str(),
            format!(
                "Couldn't strip `{}` from `{}`",
                src_dir.display(),
                src.display()
            )
            .as_str(),
        );
        let dest = dst_dir.join(dest?);
        dst_mx3.push(dest);
    }
    Ok(dst_mx3)
}

fn create_ssh_connection(config: Config) -> Result<Session, ()> {
    let mut addr = config.server_ip.to_owned();
    addr.push(':');
    addr.push_str(&config.server_port.to_string());
    let private_key = Path::new(&config.key_path);
    if !private_key.exists() {
        error!("key_file `{}` doesn't exist.", private_key.display());
        return Err(());
    }
    let mut sess = Session::new().unwrap();
    let tcp = dlog(
        TcpStream::connect(&addr),
        "TCP connection created",
        format!("Couldn't make a TCP connection to {addr}").as_str(),
    )?;
    // .map_err(|_| format!("Couldn't make a TCP connection to {addr}"))?;
    sess.set_tcp_stream(tcp);
    dlog(
        sess.handshake(),
        "Successful TCP handshake with the server.",
        "Handshake with the server failed.",
    )?;
    dlog(
        sess.userauth_pubkey_file("mat", None, private_key, None),
        format!(
            "Sucessful SSH authentication to `{}` with keyfile `{}`",
            addr,
            private_key.display()
        )
        .as_str(),
        format!(
            "SSH authentication failed to `{}` with keyfile `{}`",
            addr,
            private_key.display()
        )
        .as_str(),
    )?;
    Ok(sess)
}

fn create_dst_dir(sftp: &Sftp, dst_dir: &Path) -> Result<(), ()> {
    let stats = dlog(
        sftp.readdir(Path::new(".")),
        "Read the SFTP home directory",
        "Couldn't read the SFTP home directory",
    )?;
    let dst_dir_exists = stats.into_iter().any(|(p, _)| p == dst_dir);
    if !dst_dir_exists {
        dlog(
            sftp.mkdir(&dst_dir, 0o775),
            format!("Made `{}` on the SFTP server", dst_dir.display()).as_str(),
            format!("Couldn't make `{}` on the SFTP server", dst_dir.display()).as_str(),
        )?;
    }
    Ok(())
}

fn transfer_mx3(sftp: &Sftp, src_mx3: Vec<PathBuf>, dst_mx3: &Vec<PathBuf>) -> Result<(), ()> {
    for (src, dst) in src_mx3.into_iter().zip(dst_mx3.into_iter()) {
        let src_buf = dlog(
            fs::read(&src),
            format!("Read source file {} into buffer", src.display()).as_str(),
            format!("Couldn't read source file {} into buffer", src.display()).as_str(),
        )?;
        let mut dest_buf = dlog(
            sftp.create(&dst),
            format!("Created destination buffer {}", src.display()).as_str(),
            format!("Couldn't create destination buffer {}", src.display()).as_str(),
        )?;
        dlog(
            dest_buf.write_all(&src_buf),
            format!(
                "Wrote source file {} into destination buffer {}",
                src.display(),
                dst.display()
            )
            .as_str(),
            format!(
                "Couldn't write source file {} into destination buffer {}",
                src.display(),
                dst.display()
            )
            .as_str(),
        )?;
        info!("Transfered {}", dst.display());
    }
    Ok(())
}

fn send_command(sess: &Session, command: &String) -> Result<String, ()> {
    let mut channel = dlog(
        sess.channel_session(),
        "Created channel",
        "Error creating a channel",
    )?;
    dlog(
        channel.exec(&command),
        format!("Successfully executed `{}`", &command).as_str(),
        format!("Failed executing `{}`", &command).as_str(),
    )?;
    let mut command_output = String::new();
    dlog(
        channel.read_to_string(&mut command_output),
        format!("Output: `{}`", &command_output).as_str(),
        format!("Couldn't read output from `{}`", &command).as_str(),
    )?;
    dlog(
        channel.wait_close(),
        "Closed the channel",
        "Error closing the channel",
    )?;
    Ok(command_output)
}

fn start_jobs(sess: &Session, dst_mx3: Vec<PathBuf>) -> Result<(), ()> {
    for mx3 in dst_mx3 {
        let command = format!("sbatch {}", mx3.display());
        let s = send_command(&sess, &command)?;
        debug!("Sent command: `{}` \n  `{}`", command, s);
    }
    Ok(())
}
