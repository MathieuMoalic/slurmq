use crate::config::{self, Config};
use log::{debug, error, info};
use ssh2::{Session, Sftp};
use std::{
    fs,
    io::{Read, Write},
    net::TcpStream,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

pub fn main(config: Config, sbatch: &String, input_dir: &String, dst_dir: &String) {
    if queue(input_dir, sbatch, &config, dst_dir) == Ok(()) {
        return;
    }
}

fn queue(
    src_dir: &String,
    sbatch: &String,
    config: &config::Config,
    dst_jobs_dir: &String,
) -> Result<(), ()> {
    let sbatch_path = get_sbatch(sbatch)?;
    let src_dir = get_src_dir(src_dir)?;
    let src_mx3 = get_src_mx3(&src_dir)?;
    info!("Found these mx3 files: {:#?}", src_mx3);
    let dst_dir = get_dst_dir(&src_dir, &dst_jobs_dir)?;
    let dst_mx3 = get_dst_mx3(&src_mx3, &src_dir, &dst_dir)?;
    debug!("List of mx3 destination paths: {:#?}", dst_mx3);
    let (sess, sftp) = create_ssh_connection(config)?;
    info!("SSH connection successful");
    create_dst_dir(&sess, &dst_dir)?;
    info!("Destination directory created");
    transfer_mx3(&sftp, src_mx3, &dst_mx3)?;
    info!("All .mx3 files transfered successfully.");
    transfer_sbatch(&sftp, &sbatch_path, &dst_jobs_dir)?;
    info!("Sbatch file transfered successfully.");
    start_jobs(&sess, dst_mx3, sbatch)?;
    info!("All jobs started successfully.");
    Ok(())
}

fn get_sbatch(sbatch: &String) -> Result<PathBuf, ()> {
    let sbatch = Path::new(sbatch);
    if sbatch.exists() {
        debug!("Found {}", sbatch.display());
        return Ok(sbatch.to_path_buf());
    } else {
        error!("Sbatch file {} doesn't exist", sbatch.display());
        return Err(());
    }
}

fn get_src_dir(src_dir: &String) -> Result<PathBuf, ()> {
    let src_dir = Path::new(src_dir);
    if src_dir.exists() {
        debug!("Found input directory {}", src_dir.display());
        return Ok(src_dir.to_path_buf());
    } else {
        error!("Input directory {} doesn't exist", src_dir.display());
        return Err(());
    }
}

fn get_src_mx3(src_dir: &Path) -> Result<Vec<PathBuf>, ()> {
    if src_dir.is_dir() {
        let mut mx3_paths: Vec<PathBuf> = vec![];
        for entry in WalkDir::new(src_dir)
            .max_depth(1)
            .follow_links(true)
            .into_iter()
            .filter_map(Result::ok)
        {
            let p = entry.into_path();
            if p.is_file() {
                let extension = p.extension().unwrap_or_default();
                if extension == "mx3" {
                    mx3_paths.push(p);
                }
            }
        }
        if mx3_paths.is_empty() {
            error!("Couldn't find any .mx3 files in {}", src_dir.display());
            Err(())
        } else {
            Ok(mx3_paths)
        }
    } else {
        error!("Directory not found : {}", src_dir.display());
        Err(())
    }
}

fn get_dst_dir(src_dir: &Path, dst_jobs_dir: &String) -> Result<PathBuf, ()> {
    let parent = match src_dir.parent() {
        Some(s) => {
            debug!("Parent dir:  `{}`", s.display());
            s
        }
        None => {
            error!("Error getting the parent of `{}`", src_dir.display());
            return Err(());
        }
    };
    let dst_dir = match src_dir.strip_prefix(parent) {
        Ok(s) => {
            debug!("Source dir with stripped path: `{}`", s.display());
            s
        }
        Err(s) => {
            error!(
                "Couldn't stripped prefix: `{}` from `{}`",
                parent.display(),
                s
            );
            return Err(());
        }
    };
    let dst_dir = Path::new(&format!("{}", dst_jobs_dir)).join(dst_dir);
    debug!("Destination dir: `{}`", dst_dir.display());
    Ok(dst_dir)
}

fn get_dst_mx3(src_mx3: &Vec<PathBuf>, src_dir: &Path, dst_dir: &Path) -> Result<Vec<PathBuf>, ()> {
    let mut dst_mx3 = vec![];
    for src in src_mx3 {
        let dest = match src.strip_prefix(src_dir) {
            Ok(s) => {
                debug!(
                    "Stripped `{}` from `{}` : `{}`",
                    src_dir.display(),
                    src.display(),
                    s.display()
                );
                s
            }
            Err(s) => {
                error!(
                    "Couldn't strip `{}` from `{}` \n {s}",
                    src_dir.display(),
                    src.display()
                );
                return Err(());
            }
        };
        let dest = dst_dir.join(dest);
        dst_mx3.push(dest);
    }
    Ok(dst_mx3)
}

fn create_ssh_connection(config: &config::Config) -> Result<(Session, Sftp), ()> {
    let mut sess = match Session::new() {
        Ok(sess) => {
            debug!("Created SSH session");
            sess
        }
        Err(e) => {
            error!("Error creating the SSH session: {}", e);
            return Err(());
        }
    };
    let tcp = match TcpStream::connect(&config.addr) {
        Ok(tcp) => {
            debug!("TCP connection created");
            tcp
        }
        Err(e) => {
            error!("Couldn't make a TCP connection to {} : {}", &config.addr, e);
            return Err(());
        }
    };
    sess.set_tcp_stream(tcp);
    match sess.handshake() {
        Ok(_) => debug!("Successful TCP handshake with the server."),
        Err(_) => error!("Handshake with the server failed."),
    };
    match sess.userauth_pubkey_file(&config.user, None, &config.key, None) {
        Ok(_) => debug!(
            "Sucessful SSH authentication to `{}` with keyfile `{}`",
            &config.addr,
            &config.key.display()
        ),
        Err(_) => error!(
            "SSH authentication failed to `{}` with keyfile `{}`",
            &config.addr,
            &config.key.display()
        ),
    };
    let sftp = match sess.sftp() {
        Ok(sftp) => {
            debug!("Made SFTP connection");
            sftp
        }
        Err(e) => {
            error!("Failed making the SFTP connection");
            return Err(());
        }
    };
    Ok((sess, sftp))
}

fn create_dst_dir(sess: &Session, dst_dir: &Path) -> Result<(), ()> {
    let command = format!("mkdir -p {}", dst_dir.display());
    let stdout = send_command(sess, &command)?;
    debug!("Sent command: `{}` \n  `{}`", command, stdout);
    Ok(())
}

fn transfer_mx3(sftp: &Sftp, src_mx3: Vec<PathBuf>, dst_mx3: &[PathBuf]) -> Result<(), ()> {
    for (src, dst) in src_mx3.into_iter().zip(dst_mx3.iter()) {
        let src_buf = match fs::read(&src) {
            Ok(src_buf) => {
                debug!("Read source file {} into buffer", src.display());
                src_buf
            }
            Err(_) => {
                error!("Couldn't read source file {} into buffer", src.display());
                return Err(());
            }
        };
        let mut dest_buf = match sftp.create(dst) {
            Ok(dest_buf) => {
                debug!("Created destination buffer {}", src.display());
                dest_buf
            }
            Err(_) => {
                error!("Couldn't create destination buffer {}", src.display());
                return Err(());
            }
        };
        match dest_buf.write_all(&src_buf) {
            Ok(dest_buf) => {
                debug!(
                    "Wrote source file {} into destination buffer {}",
                    src.display(),
                    dst.display()
                );
                dest_buf
            }
            Err(_) => {
                error!(
                    "Couldn't write source file {} into destination buffer {}",
                    src.display(),
                    dst.display()
                );
                return Err(());
            }
        };
        info!("Transfered {}", dst.display());
    }
    Ok(())
}

fn transfer_sbatch(sftp: &Sftp, src: &PathBuf, dst_jobs_dir: &String) -> Result<(), ()> {
    let src_buf = match fs::read(&src) {
        Ok(src_buf) => {
            debug!("Read source file {} into buffer", src.display());
            src_buf
        }
        Err(_) => {
            error!("Couldn't read source file {} into buffer", src.display());
            return Err(());
        }
    };
    let dst = match Path::new(src).file_name() {
        Some(dst) => {
            debug!("Sbatch filename: {}", dst.to_string_lossy());
            dst.as_ref()
        }
        None => {
            error!("Couldn't get sbatch filename");
            return Err(());
        }
    };
    let mut dst_buf = match sftp.create(dst) {
        Ok(dest_buf) => {
            debug!("Created destination buffer {}", src.display());
            dest_buf
        }
        Err(_) => {
            error!("Couldn't create destination buffer {}", src.display());
            return Err(());
        }
    };
    match dst_buf.write_all(&src_buf) {
        Ok(dest_buf) => {
            debug!(
                "Wrote source file {} into destination buffer {}",
                src.display(),
                dst.display()
            );
            dest_buf
        }
        Err(_) => {
            error!(
                "Couldn't write source file {} into destination buffer {}",
                src.display(),
                dst.display()
            );
            return Err(());
        }
    };
    info!("Transfered {}", dst.display());

    Ok(())
}

fn send_command(sess: &Session, command: &String) -> Result<String, ()> {
    let mut channel = match sess.channel_session() {
        Ok(channel) => {
            debug!("Created channel");
            channel
        }
        Err(e) => {
            error!("Error creating channel: {}", e);
            return Err(());
        }
    };
    match channel.exec(command) {
        Ok(_) => {
            debug!("Successfully executed `{}`", &command);
        }
        Err(e) => {
            error!("Failed executing `{}` :{}", &command, e);
            return Err(());
        }
    };
    let mut stdout = String::new();
    match channel.read_to_string(&mut stdout) {
        Ok(_) => {
            debug!("Output: `{}`", &stdout);
        }
        Err(e) => {
            error!("Couldn't read output from `{}` : {}", &command, e);
            return Err(());
        }
    };
    match channel.wait_close() {
        Ok(_) => {
            debug!("Closed the channel");
        }
        Err(e) => {
            error!("Error closing the channel: {}", e);
            return Err(());
        }
    };
    let exit_code = match channel.exit_status() {
        Ok(exit_code) => {
            debug!("Got exit code");
            exit_code
        }
        Err(e) => {
            error!("Failed while getting exit code: {}", e);
            return Err(());
        }
    };
    if exit_code == 0 {
        Ok(stdout)
    } else {
        let mut stderr = vec![];
        let exit_code = match channel.stderr().read_to_end(&mut stderr) {
            Ok(exit_code) => {
                debug!("Read stderr");
                exit_code
            }
            Err(e) => {
                error!("Error reading stderr: {}", e);
                return Err(());
            }
        };
        match String::from_utf8(stderr) {
            Ok(stderr) => {
                error!(
                    "Command `{}` failed with exit code {}:\n {:?}",
                    &command,
                    exit_code,
                    stderr.as_str()
                );
            }
            Err(e) => {
                error!("Error reading stderr: {}", e);
                return Err(());
            }
        };
        Err(())
    }
}

fn start_jobs(sess: &Session, dst_mx3: Vec<PathBuf>, sbatch: &String) -> Result<(), ()> {
    for mx3 in dst_mx3 {
        let mx3_path = mx3.to_str().unwrap();
        let zarr_path = mx3_path.replace(".mx3", ".zarr");
        let log_path = mx3_path.replace(".mx3", ".zarr/slurm.logs");
        // let calc_log_path = mx3_path.replace(".mx3", ".zarr/slurm_calc.logs");
        let job_name = mx3.file_stem().unwrap().to_str().unwrap();
        let command = format!("mkdir -p {zarr_path}");
        let stdout = send_command(sess, &command)?;
        debug!("Sent command: `{}` \n  `{}`", command, stdout);
        let command =
            format!("sbatch --job-name={job_name} --output={log_path} {sbatch} {mx3_path}");
        let stdout = send_command(sess, &command)?;
        debug!("Sent command: `{}` \n  `{}`", command, stdout);

        // mx3_path=$PWD/$arg
        // zarr_path="${mx3_path/.mx3/.zarr}"
        // log_path=$zarr_path/slurm.logs
        // calc_log_path=$zarr_path/slurm_calc.logs
        // batch_name="$(basename "$(dirname $mx3_path)")"
        // job_name="${arg/.mx3/}"
        // mkdir -p $zarr_path
        // JobID=$(sbatch --job-name="$job_name" --output="$log_path" $HOME/sbatch/amumax.sh $mx3_path | cut -f 4 -d' ')
        // sbatch -d afterany:$JobID --job-name="calc_$job_name" --output=$calc_log_path $HOME/sbatch/amumax_post.sh $zarr_path $batch_name $calc_modes > /dev/null
        // echo " - Submitted job for ${job_name}.mx3"
    }
    Ok(())
}
