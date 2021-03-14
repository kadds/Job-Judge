use crate::table;
use log::*;
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::sync::atomic::{AtomicI32, AtomicI64, Ordering};
use std::sync::{atomic::AtomicBool, Arc};
use std::time::Duration;
use tokio::{
    sync::{broadcast, oneshot, watch, Mutex},
    time::{sleep, timeout},
};
use tonic::{Request, Response, Status};

mod id {
    pub mod rpc {
        tonic::include_proto!("id.rpc");
    }
}

use id::rpc::id_svr_server::{IdSvr, IdSvrServer};
use id::rpc::*;

const MAX_REPLICA_ID: u32 = 128;
const MAX_BIZ_ID: i32 = 128;

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
        let c = sqlx::query(
            "UPDATE biz_ids_tbl set value=$1, version=$2 where biz_id=$3 and version=$4",
        )
        .bind(biz_ids.value)
        .bind(biz_ids.version + 1)
        .bind(biz_ids.biz_id)
        .bind(biz_ids.version)
        .execute(&self.pool)
        .await?;
        if c.rows_affected() == 0 {
            Err(GenIdError::VersionFail)
        } else {
            Ok(())
        }
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
        $t.compare_exchange($old, $new, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
    };
}

async fn prefetch<T: DataSource>(
    source: Arc<T>,
    biz: i32,
    vec: Arc<[IdAllocInfo]>,
) -> Result<(), GenIdError> {
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

async fn gen_id<T: DataSource>(
    source: Arc<T>,
    biz: i32,
    vec: Arc<[IdAllocInfo]>,
) -> Result<i64, GenIdError> {
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
            info!("overflow {} {}", max, pos);
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
            sleep(Duration::from_millis(
                20 * (times - TRY_MAX_TIMES / 2 + 1) as u64,
            ))
            .await;
        }
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

pub struct IdSvrImpl<T> {
    data_source: Arc<T>,
    replica_id: u32,
    res: Arc<[IdAllocInfo]>,
}

#[tonic::async_trait]
impl<T> IdSvr for IdSvrImpl<T>
where
    T: DataSource,
{
    async fn create_id(
        &self,
        request: Request<CreateIdReq>,
    ) -> Result<Response<CreateIdRsp>, Status> {
        let req = request.into_inner();
        let id = match gen_id(self.data_source.clone(), req.biz, self.res.clone()).await {
            Ok(v) => v,
            Err(err) => {
                error!("execute sql failed when select. error {}", err);
                return Err(Status::unavailable("query database fail"));
            }
        };
        Ok(Response::new(CreateIdRsp { id }))
    }
}

pub async fn get(server: Arc<micro_service::Server>) -> IdSvrServer<IdSvrImpl<DatabaseDataSource>> {
    let replica_id = server.config().replica_id.expect("not found replica id");
    assert!(replica_id < MAX_REPLICA_ID);
    let connections: u32 = 10;
    let database_url = server
        .config()
        .comm_database
        .url
        .clone()
        .expect("not found comm database url");
    let pool = match PgPoolOptions::new()
        .max_connections(connections)
        .connect_timeout(Duration::from_secs(5))
        .connect(&database_url)
        .await
    {
        Ok(v) => v,
        Err(err) => {
            panic!("connect database err {}", err);
        }
    };
    let mut bizs = Vec::<IdAllocInfo>::new();
    bizs.resize_with(MAX_BIZ_ID as usize, || IdAllocInfo::new());
    let res: Arc<[IdAllocInfo]> = bizs.into();

    return IdSvrServer::new(IdSvrImpl {
        data_source: Arc::new(DatabaseDataSource::new(pool)),
        replica_id,
        res,
    });
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::table;
    use std::sync::Arc;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn try_id_async() {
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
        let mut bizs = Vec::new();
        bizs.push(IdAllocInfo::new());
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
}
