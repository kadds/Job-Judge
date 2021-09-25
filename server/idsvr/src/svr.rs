use crate::table;
use log::*;
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::sync::Arc;
use std::time::Duration;
use std::{
    sync::atomic::{AtomicI32, AtomicI64, Ordering},
    time::{SystemTime, SystemTimeError, UNIX_EPOCH},
};
use tokio::net::TcpListener;
use tokio::{
    sync::{broadcast, Mutex},
    time::{sleep, timeout},
};
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{transport::Server, Request, Response, Status};

mod id {
    pub mod rpc {
        tonic::include_proto!("id.rpc");
    }
}
pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("descriptor");

use id::rpc::id_svr_server::{IdSvr, IdSvrServer};
use id::rpc::*;

const MAX_BIZ_ID: i32 = 128;

const SEQ_BIT: u8 = 13;
const SEQ_RIGHT: u8 = 0;
const REPLICA_BIT: u8 = 9;
const REPLICA_RIGHT: u8 = SEQ_RIGHT + SEQ_BIT;
const TIME_BIT: u8 = 41;
const TIME_RIGHT: u8 = REPLICA_RIGHT + REPLICA_BIT;

const MAX_REPLICA_ID: u32 = 1 << REPLICA_BIT;
const START_TIMESTAMP: u64 = 1600000000;

#[derive(thiserror::Error, Debug)]
pub enum GenIdError {
    #[error("sql error {0}")]
    SqlError(#[from] sqlx::Error),
    #[error("invalid biz type")]
    InvalidBiz,
    #[error("maximum range")]
    MaximumRange,
    #[error("version check fail")]
    VersionFail,
    #[error("many times try fail")]
    ManyTimes,
}

#[tonic::async_trait]
pub trait DataSource: Send + Sync + 'static {
    async fn fetch(&self, biz: i32) -> Result<table::BizIds, GenIdError>;
    async fn save(&self, biz_ids: &table::BizIds) -> Result<(), GenIdError>;
}

pub struct DatabaseDataSource {
    pool: PgPool,
}

impl DatabaseDataSource {
    pub fn new(pool: PgPool) -> Self {
        DatabaseDataSource { pool }
    }
}

#[tonic::async_trait]
impl DataSource for DatabaseDataSource {
    async fn fetch(&self, biz: i32) -> Result<table::BizIds, GenIdError> {
        let bizid: Option<table::BizIds> =
            sqlx::query_as::<_, table::BizIds>("SELECT * from biz_ids_tbl where biz_id=$1")
                .bind(biz as i64)
                .fetch_optional(&self.pool)
                .await?;
        bizid.ok_or(GenIdError::InvalidBiz)
    }

    async fn save(&self, biz_ids: &table::BizIds) -> Result<(), GenIdError> {
        let c = sqlx::query("UPDATE biz_ids_tbl set value=$1, version=$2 where biz_id=$3 and version=$4")
            .bind(biz_ids.value)
            .bind(biz_ids.version + 1)
            .bind(biz_ids.biz_id)
            .bind(biz_ids.version)
            .execute(&self.pool)
            .await?;
        if c.rows_affected() == 0 { Err(GenIdError::VersionFail) } else { Ok(()) }
    }
}

#[cfg(test)]
pub struct MemoryDataSource {
    map: tokio::sync::Mutex<std::collections::HashMap<i32, table::BizIds>>,
}

#[cfg(test)]
impl MemoryDataSource {
    pub fn new() -> Self {
        MemoryDataSource {
            map: tokio::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

#[cfg(test)]
#[tonic::async_trait]
impl DataSource for MemoryDataSource {
    async fn fetch(&self, biz: i32) -> Result<table::BizIds, GenIdError> {
        let map = self.map.lock().await;
        let bizid = map.get(&biz);
        bizid.ok_or(GenIdError::InvalidBiz).map(|v| v.clone())
    }

    async fn save(&self, biz_ids: &table::BizIds) -> Result<(), GenIdError> {
        let mut map = self.map.lock().await;
        let bizid = map.get(&biz_ids.biz_id);
        let bizid = bizid.ok_or(GenIdError::InvalidBiz)?;
        if bizid.version == biz_ids.version {
            let id = biz_ids.biz_id;
            map.insert(id, biz_ids.clone());
            Ok(())
        } else {
            Err(GenIdError::VersionFail)
        }
    }
}

macro_rules! cas {
    ($t: expr, $old: expr, $new: expr) => {
        $t.compare_exchange($old, $new, Ordering::Acquire, Ordering::Relaxed).is_ok()
    };
}

async fn prefetch<T: DataSource>(source: Arc<T>, biz: i32, vec: Arc<[IdAllocInfo]>) -> Result<(), GenIdError> {
    let u = vec.get(biz as usize).unwrap();
    let mut bizid = match source.fetch(biz).await {
        Ok(v) => v,
        Err(err) => {
            u.updating.store(0, Ordering::SeqCst);
            return Err(err);
        }
    };
    let pos = bizid.value;
    bizid.value += bizid.step;
    match source.save(&bizid).await {
        Ok(()) => (),
        Err(err) => {
            u.updating.store(0, Ordering::SeqCst);
            return Err(err);
        }
    };

    let max = std::cmp::min(bizid.value, bizid.max_value);
    let danger = pos + bizid.step / 2;
    let biz_info = NBizIdInfo { pos, danger, max };
    let mut v = u.next_biz_info.lock().await;
    *v = biz_info;
    u.updating.store(2, Ordering::SeqCst);
    let _ = u.tx.send(());
    Ok(())
}

async fn gen_id<T: DataSource>(source: Arc<T>, biz: i32, vec: Arc<[IdAllocInfo]>) -> Result<i64, GenIdError> {
    const TRY_MAX_TIMES: usize = 20;
    if biz >= MAX_BIZ_ID {
        return Err(GenIdError::InvalidBiz);
    }
    let u = vec.get(biz as usize).unwrap();
    let mut last_max = -1;
    for times in 0..TRY_MAX_TIMES {
        let pos = u.biz_info.pos.load(Ordering::SeqCst);
        let danger = u.biz_info.danger.load(Ordering::SeqCst);
        let max = u.biz_info.max.load(Ordering::SeqCst);
        if last_max >= max {
            info!("out of range at biz {} max {} pos {}", biz, last_max, pos);
            return Err(GenIdError::MaximumRange);
        }
        if pos >= danger && cas!(u.updating, 0, 1) {
            tokio::spawn(prefetch(source.clone(), biz, vec.clone()));
        }
        if pos >= max {
            trace!("overflow {} {}", max, pos);
            let mut rx = u.tx.subscribe();
            // try pick the future after prefetching
            let _ = timeout(Duration::from_millis(0), rx.recv()).await;
            if u.updating.load(Ordering::SeqCst) == 1 {
                let _ = rx.recv().await;
            }
            if cas!(u.updating, 2, 3) {
                let new = u.next_biz_info.lock().await;
                u.biz_info.max.store(new.max, Ordering::SeqCst);
                u.biz_info.danger.store(new.danger, Ordering::SeqCst);
                u.biz_info.pos.store(new.pos, Ordering::SeqCst);
                u.updating.store(0, Ordering::SeqCst);
            }
        } else if cas!(u.biz_info.pos, pos, pos + 1) {
            return Ok(pos);
        }
        last_max = max;
        if times >= TRY_MAX_TIMES / 2 {
            sleep(Duration::from_millis(20 * (times - TRY_MAX_TIMES / 2 + 1) as u64)).await;
        }
        trace!("gen_id try next times");
    }
    Err(GenIdError::ManyTimes)
}

struct BizIdInfo {
    pos: AtomicI64,
    danger: AtomicI64,
    max: AtomicI64,
}

impl BizIdInfo {
    pub fn new() -> Self {
        BizIdInfo {
            pos: AtomicI64::new(0),
            danger: AtomicI64::new(0),
            max: AtomicI64::new(0),
        }
    }
}
#[derive(Debug, Clone, Default)]
struct NBizIdInfo {
    pos: i64,
    danger: i64,
    max: i64,
}

struct IdAllocInfo {
    biz_info: BizIdInfo,
    next_biz_info: Mutex<NBizIdInfo>,
    updating: AtomicI32,
    tx: broadcast::Sender<()>,
}

impl IdAllocInfo {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel::<()>(1);
        IdAllocInfo {
            biz_info: BizIdInfo::new(),
            next_biz_info: Mutex::new(NBizIdInfo::default()),
            updating: AtomicI32::new(0),
            tx,
        }
    }
}

struct SnowflakeData {
    last_val: AtomicI64,
    replica_id: u32,
}

impl SnowflakeData {
    pub fn new(replica_id: u32) -> Self {
        SnowflakeData {
            last_val: AtomicI64::new(0),
            replica_id,
        }
    }
}

pub struct IdSvrImpl<T> {
    data_source: Arc<T>,
    res: Arc<[IdAllocInfo]>,
    data: SnowflakeData,
}

#[derive(thiserror::Error, Debug)]
pub enum GenSnowflakeSeqError {
    #[error("time cast fail")]
    TimeCastFail(#[from] SystemTimeError),
    #[error("out of range")]
    OutOfRange,
    #[error("time travel")]
    TimeTravel,
    #[error("many times try fail")]
    ManyTimes,
}

const fn from_bits(v: i64, bits: u8, right: u8) -> i64 {
    (v >> right) & ((1 << bits) - 1)
}

//
//  | 41 bits time(ms) | 9 bits replica_id | 13 bits seq |
//
async fn gen_snowflake_seq(data: &SnowflakeData) -> Result<i64, GenSnowflakeSeqError> {
    const TRY_MAX_TIMES: usize = 50;
    let e = UNIX_EPOCH + Duration::from_secs(START_TIMESTAMP);
    for times in 0..TRY_MAX_TIMES {
        let delta = match SystemTime::now().duration_since(e) {
            Ok(v) => v.as_millis() as i64,
            Err(err) => {
                return Err(GenSnowflakeSeqError::TimeCastFail(err));
            }
        };

        let last = data.last_val.load(Ordering::SeqCst);
        let last_delta = from_bits(last, TIME_BIT, TIME_RIGHT);
        let last_seq = from_bits(last, SEQ_BIT, SEQ_RIGHT);
        let d = last_delta - delta;
        #[allow(clippy::comparison_chain)]
        if d == 0 {
            if last_seq < (1 << SEQ_BIT) - 1 {
                if cas!(data.last_val, last, last + 1) {
                    return Ok(last + 1);
                }
            } else {
                trace!("sold out");
                // seq sold out, next millis
                sleep(Duration::from_millis(1)).await;
            }
        } else if d > 0 {
            // time travel appear, 100ms maximum
            if d <= 100 {
                trace!("time travel");
                sleep(Duration::from_millis(d as u64)).await;
            } else {
                return Err(GenSnowflakeSeqError::TimeTravel);
            }
        } else {
            // d < 0
            // last_delta < delta
            if delta >= (1 << TIME_BIT) {
                return Err(GenSnowflakeSeqError::OutOfRange);
            }
            let mut result = delta << TIME_RIGHT;
            result |= (data.replica_id as i64) << REPLICA_RIGHT;
            if cas!(data.last_val, last, result) {
                return Ok(result);
            }
        }
        if times >= TRY_MAX_TIMES / 2 {
            sleep(Duration::from_millis((times - TRY_MAX_TIMES / 2 + 1) as u64)).await;
        }
        trace!("gen_snowflake try next times");
    }
    Err(GenSnowflakeSeqError::ManyTimes)
}

#[tonic::async_trait]
impl<T> IdSvr for IdSvrImpl<T>
where
    T: DataSource,
{
    async fn create_id(&self, request: Request<CreateIdReq>) -> Result<Response<CreateIdRsp>, Status> {
        let req = request.into_inner();
        let id = match gen_id(self.data_source.clone(), req.biz, self.res.clone()).await {
            Ok(v) => v,
            Err(err) => {
                error!("execute sql failed when select. error {}", err);
                return Err(Status::unavailable("fail"));
            }
        };
        Ok(Response::new(CreateIdRsp { id }))
    }

    async fn create_seq(&self, _request: Request<CreateSeqReq>) -> Result<Response<CreateSeqRsp>, Status> {
        let id = match gen_snowflake_seq(&self.data).await {
            Ok(v) => v,
            Err(err) => {
                error!("gen snowflake seq fail. error {}", err);
                return Err(Status::unavailable("fail"));
            }
        };
        Ok(Response::new(CreateSeqRsp { id }))
    }
}

pub async fn get(server: Arc<micro_service::Server>, listener: TcpListener) {
    let replica_id = server.config().replica_id.expect("not found replica id");
    assert!(replica_id < MAX_REPLICA_ID);
    let connections: u32 = 10;
    let database_url = server.config().comm_database.url.clone().expect("not found comm database url");

    let pool = PgPoolOptions::new()
        .max_connections(connections)
        .connect_timeout(Duration::from_secs(5))
        .connect(&database_url)
        .await
        .expect("connect database fail");

    let mut bizs = Vec::<IdAllocInfo>::new();
    bizs.resize_with(MAX_BIZ_ID as usize, IdAllocInfo::new);
    let res: Arc<[IdAllocInfo]> = bizs.into();

    let svr = IdSvrServer::new(IdSvrImpl::<DatabaseDataSource> {
        data_source: Arc::new(DatabaseDataSource::new(pool)),
        res,
        data: SnowflakeData::new(replica_id),
    });

    let reflection_svr = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
        .build()
        .unwrap();

    Server::builder()
        .add_service(svr)
        .add_service(reflection_svr)
        .serve_with_incoming_shutdown(TcpListenerStream::new(listener), server.wait_stop_signal())
        .await
        .expect("start server fail");
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::table;
    use std::{collections::HashSet, sync::Arc};
    const THREADS: u32 = 10;
    const CNT: u32 = 1000;

    #[test]
    fn from_bits_test() {
        assert_eq!(from_bits(0b100101, 2, 0), 0b1);
        assert_eq!(from_bits(0b100101, 3, 2), 0b1);
        assert_eq!(from_bits(0b111101, 2, 3), 0b11);
        assert_eq!(from_bits(0b100101, 1, 63), 0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn try_id_out_of_range() {
        let _ = env_logger::try_init();
        let s = Arc::new(MemoryDataSource::new());
        let mut map = s.map.lock().await;
        map.insert(
            0,
            table::BizIds {
                biz_id: 0,
                step: 3,
                value: 1000,
                version: 0,
                max_value: 1005,
                bak_value: 0,
            },
        );
        drop(map);
        let bizs = vec![IdAllocInfo::new()];
        let b: Arc<[IdAllocInfo]> = bizs.into();
        assert_eq!(gen_id(s.clone(), 0, b.clone()).await.unwrap(), 1000);
        assert_eq!(gen_id(s.clone(), 0, b.clone()).await.unwrap(), 1001);
        assert_eq!(gen_id(s.clone(), 0, b.clone()).await.unwrap(), 1002);
        assert_eq!(gen_id(s.clone(), 0, b.clone()).await.unwrap(), 1003);
        assert_eq!(gen_id(s.clone(), 0, b.clone()).await.unwrap(), 1004);
        assert!(gen_id(s.clone(), 0, b.clone()).await.is_err());
        assert!(gen_id(s.clone(), 0, b.clone()).await.is_err());
        assert!(gen_id(s.clone(), 0, b.clone()).await.is_err());
    }

    fn valid_seqs(seqs: &Vec<(i64, u32)>, start_ts: SystemTime, end_ts: SystemTime) {
        let mut map = HashSet::<i64>::new();
        let e = UNIX_EPOCH + Duration::from_secs(START_TIMESTAMP);
        let start_ts = start_ts.duration_since(e).unwrap().as_millis() as i64;
        let end_ts = end_ts.duration_since(e).unwrap().as_millis() as i64;
        for (seq, replica_id) in seqs {
            let v = *seq;
            assert!(map.insert(v));
            let delta = from_bits(v, TIME_BIT, TIME_RIGHT);
            let id = from_bits(v, REPLICA_BIT, REPLICA_RIGHT) as u32;
            assert_eq!(id, *replica_id);
            assert!(delta >= start_ts && delta <= end_ts);
        }
    }

    async fn gen_fn(times: u32, vec: Arc<Mutex<Vec<(i64, u32)>>>, data: SnowflakeData) {
        let mut tmp = vec![];
        for _ in 0..times {
            tmp.push(gen_snowflake_seq(&data).await.unwrap());
        }
        let mut vec = vec.lock().await;
        for v in tmp {
            vec.push((v, data.replica_id));
        }
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 5)]
    async fn try_seq_multi_thread() {
        let _ = env_logger::try_init();
        let start_ts = SystemTime::now();
        let vec = Arc::new(Mutex::new(Vec::<(i64, u32)>::new()));
        let mut f = Vec::new();
        for replica_id in 0..THREADS {
            let data = SnowflakeData::new(replica_id);
            f.push(tokio::spawn(gen_fn(CNT as u32, vec.clone(), data)));
        }
        let _ = futures::future::join_all(f).await;
        let end_ts = SystemTime::now();
        let v = vec.lock().await;
        assert_eq!(v.len(), (CNT * THREADS) as usize);
        valid_seqs(&v, start_ts, end_ts);
    }
}
