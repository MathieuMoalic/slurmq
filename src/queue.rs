use crate::{config, logging::dlog, REMOTE_JOB_DIR, SBATCH1_PATH};
use log::{debug, error, info};
use ssh2::{Session, Sftp};
use std::{
    fs,
    io::{Read, Write},
    net::TcpStream,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

pub fn main(src_dir: &String, host: &String) {
    let conf = if let Ok(conf) = config::load(host.to_string()) {
        conf
    } else {
        debug!("Failed loading the SSH config");
        return;
    };
    if let Ok(()) = queue(src_dir, &conf) {}
}
pub fn queue(src_dir: &String, config: &config::Config) -> Result<(), ()> {
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
    create_dst_dir(&sess, &dst_dir)?;
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

fn get_dst_dir(src_dir: &Path) -> PathBuf {
    let dst_dir = src_dir.strip_prefix(src_dir.parent().unwrap()).unwrap();
    let dst_dir = Path::new(&format!("./{REMOTE_JOB_DIR}")).join(dst_dir);
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

fn create_ssh_connection(config: &config::Config) -> Result<Session, ()> {
    let mut sess = Session::new().unwrap();
    let tcp = dlog(
        TcpStream::connect(&config.addr),
        "TCP connection created",
        format!("Couldn't make a TCP connection to {}", &config.addr).as_str(),
    )?;
    sess.set_tcp_stream(tcp);
    dlog(
        sess.handshake(),
        "Successful TCP handshake with the server.",
        "Handshake with the server failed.",
    )?;
    dlog(
        sess.userauth_pubkey_file(&config.user, None, &config.key, None),
        format!(
            "Sucessful SSH authentication to `{}` with keyfile `{}`",
            &config.addr,
            &config.key.display()
        )
        .as_str(),
        format!(
            "SSH authentication failed to `{}` with keyfile `{}`",
            &config.addr,
            &config.key.display()
        )
        .as_str(),
    )?;
    Ok(sess)
}

fn create_dst_dir(sess: &Session, dst_dir: &Path) -> Result<(), ()> {
    let command = format!("mkdir -p {}", dst_dir.display());
    let stdout = send_command(sess, &command)?;
    debug!("Sent command: `{}` \n  `{}`", command, stdout);
    Ok(())
}

fn transfer_mx3(sftp: &Sftp, src_mx3: Vec<PathBuf>, dst_mx3: &[PathBuf]) -> Result<(), ()> {
    for (src, dst) in src_mx3.into_iter().zip(dst_mx3.iter()) {
        let src_buf = dlog(
            fs::read(&src),
            format!("Read source file {} into buffer", src.display()).as_str(),
            format!("Couldn't read source file {} into buffer", src.display()).as_str(),
        )?;
        let mut dest_buf = dlog(
            sftp.create(dst),
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
        channel.exec(command),
        format!("Successfully executed `{}`", &command).as_str(),
        format!("Failed executing `{}`", &command).as_str(),
    )?;
    let mut stdout = String::new();
    dlog(
        channel.read_to_string(&mut stdout),
        format!("Output: `{}`", &stdout).as_str(),
        format!("Couldn't read output from `{}`", &command).as_str(),
    )?;

    dlog(
        channel.wait_close(),
        "Closed the channel",
        "Error closing the channel",
    )?;
    let exit_code = dlog(
        channel.exit_status(),
        "Got exit code",
        "Failed while getting exit code",
    )?;
    if exit_code == 0 {
        Ok(stdout)
    } else {
        let mut stderr = vec![];
        channel.stderr().read_to_end(&mut stderr).unwrap();
        let stderr = String::from_utf8(stderr).unwrap();
        error!(
            "Command `{}` failed with exit code {}:\n {:?}",
            &command, exit_code, stderr
        );
        Err(())
    }
}

fn start_jobs(sess: &Session, dst_mx3: Vec<PathBuf>) -> Result<(), ()> {
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
            format!("sbatch --job-name={job_name} --output={log_path} {SBATCH1_PATH} {mx3_path}");
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
