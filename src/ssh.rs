use anyhow::{anyhow, Context, Result};
use ssh2::Session;
use ssh2_config::SshConfig;
use std::{
    fs::File,
    io::{BufReader, Read},
    net::TcpStream,
    path::PathBuf,
};

pub struct Ssh {
    pub host: String,
    pub addr: String,
    pub user: String,
    pub key: PathBuf,
    pub session: Session,
}

impl Ssh {
    pub fn new(host: &str, config_path: &str) -> Result<Self> {
        let p = expanduser::expanduser(config_path)
            .with_context(|| format!("Couldn't expand the SSH config path `{config_path}`"))?;
        let mut reader = BufReader::new(
            File::open(&p)
                .with_context(|| format!("Could not open config file at `{}`", p.display()))?,
        );
        let config = SshConfig::default().parse(&mut reader).with_context(|| {
            format!(
                "Failed to parse the configuration file at `{}` ",
                config_path
            )
        })?;
        let host_config = config.query(host);
        let mut addr = host_config
            .host_name
            .with_context(|| format!("`{}` was not found in the ssh config file", &host))?;
        let port = host_config.port.map_or_else(|| 22, |port| port);
        addr.push(':');
        addr.push_str(&port.to_string());
        let user = host_config
            .user
            .with_context(|| "`User` was not found in the ssh config file")?;
        let keys = host_config.identity_file.with_context(|| {
            format!(
                "No key file was found for `{}` in the ssh config file",
                &host
            )
        })?;
        let key = keys.first().with_context(|| {
            format!(
                "No key file was found for `{}` in the ssh config file",
                &host
            )
        })?;
        let mut sess = Session::new().with_context(|| "Error creating the SSH session")?;
        let tcp = TcpStream::connect(&addr)
            .with_context(|| format!("Couldn't make a TCP connection to {}", &addr))?;
        sess.set_tcp_stream(tcp);
        sess.handshake()
            .with_context(|| "Handshake with the server failed")?;
        sess.userauth_pubkey_file(&user, None, key, None)
            .with_context(|| {
                format!(
                    "SSH authentication failed to `{}` with keyfile `{}`",
                    addr,
                    key.display()
                )
            })?;
        Ok(Self {
            host: host.to_string(),
            addr,
            user,
            key: key.clone(),
            session: sess,
        })
    }
    pub fn send_command(&self, command: &String) -> Result<String> {
        let mut channel = self
            .session
            .channel_session()
            .with_context(|| format!("Error creating channel for the command `{command}`"))?;
        channel
            .exec(command)
            .with_context(|| format!("Failed to execute the command `{}`", &command))?;
        let mut stdout = String::new();
        channel
            .read_to_string(&mut stdout)
            .with_context(|| format!("Couldn't read output from `{}`", &command))?;
        channel
            .wait_close()
            .with_context(|| "Error closing the channel")?;
        let exit_code = channel
            .exit_status()
            .with_context(|| "Failed while getting exit code")?;
        if exit_code == 0 {
            Ok(stdout)
        } else {
            let mut stderr = vec![];
            let exit_code = channel.stderr().read_to_end(&mut stderr).with_context(|| {
                format!(
                    "The command `{}` failed and there was an error reading stderr",
                    command
                )
            })?;
            let stderr = String::from_utf8(stderr).with_context(|| "Error reading stderr")?;
            Err(anyhow!(
                "Got exit code {} with error : {}",
                exit_code,
                stderr
            ))
        }
    }
}
