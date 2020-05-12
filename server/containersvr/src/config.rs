use log::warn;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct ContainerConfig {
    pub container: String,
    pub mem_limit: u32,
    pub vcpu_cnt: u16,
    pub vcpu_percent: u8,
    pub io_speed_limit: u32,
    pub img_src: String,
    pub img_fs_src: String,
}

lazy_static! {
    static ref CFG_MAP: HashMap<String, Arc<ContainerConfig>> = HashMap::new();
}

pub fn from(filename: String) -> Option<Arc<ContainerConfig>> {
    if let Some(val) = CFG_MAP.get(&filename) {
        Some(val);
    }
    None
}

pub fn load(cfgs: Vec<String>) {
    for it in cfgs {
        let mut buf = vec![];
        if File::open(it).and_then(|f| f.read_to_end(&mut buf)).is_ok() {
            if toml::from_slice(&buf)
                .ok()
                .and_then(|c: ContainerConfig| CFG_MAP.insert(it, Arc::from(c)))
                .is_none()
            {
                warn!("config load failed {}", it);
            }
        } else {
            warn!("Unknown config path {}", it);
        }
    }
}
