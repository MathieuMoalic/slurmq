use std::{fs::File, io::BufReader, path::PathBuf};

use ssh2_config::SshConfig;

use crate::{logging::dlog, SSH_CONFIG_PATH};

#[derive(Debug)]
pub struct Config {
    pub addr: String,
    pub user: String,
    pub key: PathBuf,
}

pub fn load(host: String) -> Result<Config, ()> {
    let p = dlog(
        expanduser::expanduser(SSH_CONFIG_PATH),
        "Got SSH config path",
        "Couldn't expand the SSH config path",
    )?;
    let mut reader = match File::open(&p) {
        Ok(f) => BufReader::new(f),
        Err(err) => panic!("Could not open file '{}': {err}", p.display()),
    };
    let config = match SshConfig::default().parse(&mut reader) {
        Ok(config) => config,
        Err(err) => panic!("Failed to parse configuration: {err}"),
    };
    let host_config = config.query(host);
    let mut addr = host_config.host_name.unwrap();
    let port = host_config.port.unwrap_or(22);
    addr.push(':');
    addr.push_str(&port.to_string());
    let user = host_config.user.unwrap();
    let keys = host_config.identity_file.unwrap();
    let key = keys.first().unwrap().clone();
    Ok(Config { addr, user, key })
}
