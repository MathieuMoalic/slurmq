use log::error;
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub server_ip: String,
    pub server_port: u32,
    pub key_path: String,
}

/// `MyConfig` implements `Default`
impl ::std::default::Default for Config {
    fn default() -> Self {
        Self {
            server_ip: "eagle.man.poznan.pl".into(),
            server_port: 22,
            key_path: "~/.ssh/id_rsa".into(),
        }
    }
}

pub fn load_config() -> Result<Config, confy::ConfyError> {
    confy::load("pcss", "pcss")
}
// pub fn save_config() -> Result<(), ::std::io::Error> {
//     let cfg = Config {
//         server_ip: "eagle.man.poznan.pl".into(),
//         server_port: 22,
//         key_path: "~/.ssh/id_rsa".into(),
//     };
//     confy::store("pcss", "pcss", cfg).unwrap();
//     Ok(())
// }
