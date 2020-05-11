extern crate serde;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Server {
    ip: String,
    port: u16,
    nic: String,
}

pub fn load() {}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
