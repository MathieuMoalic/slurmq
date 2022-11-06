use log::{debug, error};

pub fn dlog<T, E>(res: Result<T, E>, ok_msg: &str, err_msg: &str) -> Result<T, ()> {
    if res.is_ok() {
        debug!("{}", ok_msg)
    } else {
        error!("{}", err_msg)
    }
    res.map_err(|_| ())
}
