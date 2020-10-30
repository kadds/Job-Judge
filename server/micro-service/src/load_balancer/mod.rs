use std::collections::{HashMap, HashSet};

pub enum ServerChangeType {
    Add,
    Remove,
}

pub trait LoadBalancer {
    fn get_server(&self, uin: u64, flags: u64) -> Option<String>;
    fn on_server_change(&mut self, s: Vec<(String, String)>, change_type: ServerChangeType);
}

