use std::{fs::File, io::BufReader, path::PathBuf};

use log::{debug, error};
use ssh2_config::SshConfig;

#[derive(Debug)]
pub struct Config {
    pub addr: String,
    pub user: String,
    pub key: PathBuf,
}

pub fn load(host: &str, config_path: &str) -> Result<Config, ()> {
    let p = match expanduser::expanduser(config_path) {
        Ok(p) => {
            debug!("Got SSH config path");
            p
        }
        Err(e) => {
            error!(
                "Couldn't expand the SSH config path `{}` :{}",
                config_path, e
            );
            return Err(());
        }
    };
    let mut reader = match File::open(&p) {
        Ok(f) => {
            debug!("Opened the SSH config file: `{}`", &p.display());
            BufReader::new(f)
        }
        Err(err) => {
            error!("Could not open file '{}': {err}", p.display());
            return Err(());
        }
    };
    let config = match SshConfig::default().parse(&mut reader) {
        Ok(config) => {
            debug!("Parsed the SSH config file");
            config
        }
        Err(err) => {
            error!("Failed to parse configuration: {err}");
            return Err(());
        }
    };
    let host_config = config.query(host);
    let mut addr = if let Some(addr) = host_config.host_name {
        debug!("SSH server address: {}", addr);
        addr
    } else {
        error!("`{}` was not found in the ssh config file", &host);
        return Err(());
    };
    let port = host_config.port.map_or_else(
        || {
            debug!("Port was not found in the ssh config file, using default: 22");
            22
        },
        |port| {
            debug!("SSH port: {}", port);
            port
        },
    );
    addr.push(':');
    addr.push_str(&port.to_string());
    let user = if let Some(user) = host_config.user {
        debug!("SSH user: {}", user);
        user
    } else {
        error!("`User` was not found in the ssh config file");
        return Err(());
    };
    let keys = if let Some(keys) = host_config.identity_file {
        debug!("SSH keys: {:?}", keys);
        keys
    } else {
        error!(
            "No key file was found for `{}` in the ssh config file",
            &host
        );
        return Err(());
    };
    let key = if let Some(key) = keys.first() {
        debug!("Using the first key: {}", key.display());
        key.clone()
    } else {
        error!(
            "No key file was found for `{}` in the ssh config file",
            &host
        );
        return Err(());
    };
    Ok(Config { addr, user, key })
}
