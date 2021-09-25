use lazy_static::*;
use rand::prelude::*;
use std::collections::HashMap;
use std::time::SystemTime;
use tokio::sync::Mutex;

static STRMAP: &str = "1234567890abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ=+_";

lazy_static! {
    static ref HASHMAP: Mutex<HashMap<String, u64>> = Mutex::new(HashMap::new());
}

const MAXTIMEAVL: u64 = 60 * 60 * 12;

pub async fn create() -> (String, u64) {
    loop {
        let mut rng = rand::thread_rng();
        let len = rng.gen_range(30..45);
        let token: String = STRMAP.chars().choose_multiple(&mut rng, len).into_iter().collect();
        let ctime: u64 = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(0, |v| v.as_secs());
        let mut map = HASHMAP.lock().await;
        if !map.contains_key(&token) {
            map.insert(token.clone(), ctime);
            return (token, ctime);
        }
    }
}

pub async fn is_valid(token: &str) -> bool {
    if token.is_empty() {
        return false;
    }
    let mut map = HASHMAP.lock().await;
    if let Some(time) = map.get(token) {
        if SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            > MAXTIMEAVL + time
        {
            map.remove(token);
            false
        } else {
            true
        }
    } else {
        false
    }
}
