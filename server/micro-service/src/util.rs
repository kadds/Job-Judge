use rand::{prelude::*, thread_rng};

use std::time::{SystemTime, UNIX_EPOCH};

pub fn current_ts() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |v| v.as_millis() as u64)
}

pub fn gen_nid() -> u64 {
    let ts = current_ts();
    ts << 32 | thread_rng().next_u32() as u64
}

pub fn gen_tid() -> u64 {
    let ts = current_ts();
    ts << 32 | thread_rng().next_u32() as u64
}
