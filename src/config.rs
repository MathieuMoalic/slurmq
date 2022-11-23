use std::{fs::File, io::BufReader, path::PathBuf};

use log::{debug, error};
use ssh2_config::SshConfig;

#[derive(Debug)]
pub struct Config {
    pub addr: String,
    pub user: String,
    pub key: PathBuf,
}

pub fn load(host: String, config_path: String) -> Result<Config, ()> {
    let p = match expanduser::expanduser(&config_path) {
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
    let host_config = config.query(&host);
    let mut addr = match host_config.host_name {
        Some(addr) => {
            debug!("SSH server address: {}", addr);
            addr
        }
        None => {
            error!("`{}` was not found in the ssh config file", &host);
            return Err(());
        }
    };
    let port = match host_config.port {
        Some(port) => {
            debug!("SSH port: {}", port);
            port
        }
        None => {
            debug!("Port was not found in the ssh config file, using default: 22");
            22
        }
    };
    addr.push(':');
    addr.push_str(&port.to_string());
    let user = match host_config.user {
        Some(user) => {
            debug!("SSH user: {}", user);
            user
        }
        None => {
            error!("`User` was not found in the ssh config file");
            return Err(());
        }
    };
    let keys = match host_config.identity_file {
        Some(keys) => {
            debug!("SSH keys: {:?}", keys);
            keys
        }
        None => {
            error!(
                "No key file was found for `{}` in the ssh config file",
                &host
            );
            return Err(());
        }
    };
    let key = match keys.first() {
        Some(key) => {
            debug!("Using the first key: {}", key.display());
            key.clone()
        }
        None => {
            error!(
                "No key file was found for `{}` in the ssh config file",
                &host
            );
            return Err(());
        }
    };
    Ok(Config { addr, user, key })
}
