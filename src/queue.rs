use crate::ssh::Ssh;
use anyhow::{anyhow, Context, Result};
use ssh2::Sftp;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

pub fn main(ssh: &Ssh, src_dir: &String, sbatch: &String, dst_jobs_dir: &String) -> Result<()> {
    let sbatch_path = get_sbatch(sbatch)?;
    let src_dir = get_src_dir(src_dir)?;
    let src_mx3 = get_src_mx3(&src_dir)?;
    let dst_dir = get_dst_dir(&src_dir, dst_jobs_dir)?;
    let dst_mx3 = get_dst_mx3(&src_mx3, &src_dir, &dst_dir)?;
    let sess = &ssh.session;
    let sftp = sess.sftp()?;
    create_dst_dir(ssh, &dst_dir)?;
    transfer_mx3(&sftp, src_mx3, &dst_mx3)?;
    transfer_sbatch(&sftp, &sbatch_path, dst_jobs_dir)?;
    start_jobs(ssh, dst_mx3, sbatch)?;
    Ok(())
}

fn get_sbatch(sbatch: &String) -> Result<PathBuf> {
    let sbatch = Path::new(sbatch);
    if sbatch.exists() {
        Ok(sbatch.to_path_buf())
    } else {
        Err(anyhow!("Sbatch file {} doesn't exist", sbatch.display()))
    }
}

fn get_src_dir(src_dir: &String) -> Result<PathBuf> {
    let src_dir = Path::new(src_dir);
    if src_dir.exists() {
        Ok(src_dir.to_path_buf())
    } else {
        Err(anyhow!(
            "Input directory {} doesn't exist",
            src_dir.display()
        ))
    }
}

fn get_src_mx3(src_dir: &Path) -> Result<Vec<PathBuf>> {
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
            Err(anyhow!(
                "Couldn't find any .mx3 files in {}",
                src_dir.display()
            ))
        } else {
            Ok(mx3_paths)
        }
    } else {
        Err(anyhow!("Directory not found : {}", src_dir.display()))
    }
}

fn get_dst_dir(src_dir: &Path, dst_jobs_dir: &String) -> Result<PathBuf> {
    let parent = if let Some(s) = src_dir.parent() {
        format!("Parent dir: `{}`", s.display());
        s
    } else {
        return Err(anyhow!(
            "Error getting the parent of `{}`",
            src_dir.display()
        ));
    };
    let dst_dir = src_dir.strip_prefix(parent).with_context(|| {
        format!(
            "Couldn't stripped prefix: `{}` from `{}`",
            parent.display(),
            src_dir.display()
        )
    })?;
    let dst_dir = Path::new(&dst_jobs_dir.to_string()).join(dst_dir);
    Ok(dst_dir)
}

fn get_dst_mx3(src_mx3: &Vec<PathBuf>, src_dir: &Path, dst_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut dst_mx3 = vec![];
    for src in src_mx3 {
        let dest = src.strip_prefix(src_dir).with_context(|| {
            format!(
                "Couldn't strip `{}` from `{}`",
                src_dir.display(),
                src.display()
            )
        })?;
        let dest = dst_dir.join(dest);
        dst_mx3.push(dest);
    }
    Ok(dst_mx3)
}

fn create_dst_dir(ssh: &Ssh, dst_dir: &Path) -> Result<()> {
    let command = format!("mkdir -p {}", dst_dir.display());
    let stdout = ssh.send_command(&command)?;
    format!("Sent command: `{command}` \n  `{stdout}`");
    Ok(())
}

fn transfer_mx3(sftp: &Sftp, src_mx3: Vec<PathBuf>, dst_mx3: &[PathBuf]) -> Result<()> {
    for (src, dst) in src_mx3.into_iter().zip(dst_mx3.iter()) {
        let src_buf = fs::read(&src)
            .with_context(|| format!("Couldn't read source file {} into buffer", src.display()))?;
        let mut dest_buf = sftp
            .create(dst)
            .with_context(|| format!("Couldn't create destination buffer {}", src.display()))?;
        dest_buf.write_all(&src_buf).with_context(|| {
            format!(
                "Couldn't write source file {} into destination buffer {}",
                src.display(),
                dst.display()
            )
        })?;
    }
    Ok(())
}

fn transfer_sbatch(sftp: &Sftp, src: &PathBuf, _dst_jobs_dir: &str) -> Result<()> {
    let src_buf = fs::read(src)
        .with_context(|| format!("Couldn't read source file {} into buffer", src.display()))?;
    let dst = Path::new(src)
        .file_name()
        .with_context(|| "Couldn't get sbatch filename")?;
    let mut dst_buf = sftp
        .create(dst.as_ref())
        .with_context(|| format!("Couldn't create destination buffer {}", src.display()))?;
    dst_buf.write_all(&src_buf).with_context(|| {
        format!(
            "Couldn't write source file {} into destination buffer",
            src.display(),
        )
    })?;
    Ok(())
}

fn start_jobs(ssh: &Ssh, dst_mx3: Vec<PathBuf>, sbatch: &String) -> Result<()> {
    for mx3 in dst_mx3 {
        let mx3_path = mx3.to_str().ok_or_else(|| anyhow!("wda"))?;
        let zarr_path = mx3_path.replace(".mx3", ".zarr");
        let log_path = mx3_path.replace(".mx3", ".zarr/slurm.logs");
        // let calc_log_path = mx3_path.replace(".mx3", ".zarr/slurm_calc.logs");
        let job_name = mx3
            .file_name()
            .ok_or_else(|| anyhow!("Error getting the file name from {}", mx3.display()))?;
        let job_name = job_name.to_str().ok_or_else(|| {
            anyhow!(
                "Error getting turning the file name to a string {}",
                mx3.display()
            )
        })?;
        let command = format!("mkdir -p {zarr_path}");
        let _stdout = ssh.send_command(&command)?;
        let command =
            format!("sbatch --job-name={job_name} --output={log_path} {sbatch} {mx3_path}");
        let _stdout = ssh.send_command(&command)?;

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
