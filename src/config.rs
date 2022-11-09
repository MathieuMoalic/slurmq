use std::{
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};

use ssh2_config::SshConfig;

use crate::{logging::dlog, SSH_CONFIG_PATH};

#[derive(Debug)]
pub struct Config {
    pub addr: String,
    pub user: String,
    pub key: PathBuf,
}

pub fn read_config(p: &Path) -> SshConfig {
    let mut reader = match File::open(p) {
        Ok(f) => BufReader::new(f),
        Err(err) => panic!("Could not open file '{}': {}", p.display(), err),
    };
    match SshConfig::default().parse(&mut reader) {
        Ok(config) => config,
        Err(err) => panic!("Failed to parse configuration: {}", err),
    }
}

pub fn load_config(host: String) -> Result<Config, ()> {
    let p = expanduser::expanduser(SSH_CONFIG_PATH).unwrap();
    let mut reader = match File::open(&p) {
        Ok(f) => BufReader::new(f),
        Err(err) => panic!("Could not open file '{}': {}", p.display(), err),
    };
    let config = match SshConfig::default().parse(&mut reader) {
        Ok(config) => config,
        Err(err) => panic!("Failed to parse configuration: {}", err),
    };
    let host_config = config.query(host);
    let mut addr = host_config.host_name.unwrap();
    let port = host_config.port.unwrap_or(22);
    addr.push(':');
    addr.push_str(&port.to_string());
    let user = host_config.user.unwrap();
    let keys = host_config.identity_file.unwrap();
    let key = keys.first().unwrap().to_owned();
    Ok(Config { addr, user, key })
}
