use micro_service::{log, service::MicroService};
use sha2::{Sha256, Digest};
use rand::prelude::*;
use std::sync::Arc;
use std::time::Duration;
use sqlx::postgres::{PgPoolOptions, PgPool};
use sqlx::Row;
use tonic::{Request, Response, Status};
use crate::table;

type SqlRow = sqlx::postgres::PgRow;
pub mod user {
    pub mod rpc {
        tonic::include_proto!("user.rpc");
    }
    tonic::include_proto!("user");
}

use user::rpc::user_svr_server::{UserSvr, UserSvrServer};
use user::rpc::*;

pub struct UserSvrImpl {
    pool: PgPool,
    micro_service: Arc<MicroService>,
}

const CHARS: &str = "1234567890abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ=+_";
const PEPPER: &str = "&cv.98SKbSadfd=a8Dz0=";

fn random_salt() -> String {
    let mut rng = rand::thread_rng();
    let len = rng.gen_range(25,30);
    CHARS.chars().choose_multiple(&mut rng, len).into_iter().collect()
}

fn make_password_crypto(pwd: &str, salt: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(salt);
    hasher.update(pwd);
    hasher.update(PEPPER);
    hex::encode(hasher.finalize())
}

#[tonic::async_trait]
impl UserSvr for UserSvrImpl {
    async fn create_user(
        &self,
        request: Request<CreateUserReq>,
    ) -> Result<Response<CreateUserRsp>, Status> {
        let req = request.into_inner();
        let salt = random_salt();
        let pwd = req.password;
        if pwd.len() < 6 {
            return Err(Status::invalid_argument("password length too short"));
        }

        let pwd_crypto = make_password_crypto(&pwd, &salt);

        let id: i32 = match sqlx::query("INSERT INTO user_tbl (username, password, salt, nickname) VALUES 
                ($1, $2, $3, $4)
        RETURNING id")
            .bind(&req.username)
            .bind(&pwd_crypto)
            .bind(&salt)
            .bind(&req.username)
            .fetch_one(&self.pool)
            .await
        {
            Ok(v) => v.get(0),
            Err(err) => {
                error!("execute sql failed when insert. err {}", err);
                return Err(Status::unavailable("execute sql failed"));
            }
        };

        Ok(Response::new(CreateUserRsp { uid: id as u64 }))
    }

    async fn valid_user(
        &self,
        request: Request<ValidUserReq>,
    ) -> Result<Response<ValidUserRsp>, Status> {

        let req = request.into_inner();
        let res: Option<SqlRow> = match sqlx::query(
            "SELECT vid, password, salt from user_tbl where username=$1")
            .bind(&req.username)
            .fetch_optional(&self.pool)
            .await
        {
            Ok(v) => v,
            Err(err) => {
                error!("execute sql failed when select. err {}", err);
                return Err(Status::unavailable("execute sql failed"));
            }
        };

        if let Some(res) = res {
            let pwd_db: String = res.get(1);
            let salt: String = res.get(2);
            let pwd_crypto = make_password_crypto(&req.password, &salt);
            if pwd_db != pwd_crypto {
                Ok(Response::new(ValidUserRsp {
                    correct: false,
                    vid: 0
                }))
            } else {
                let vid: i64 = res.get(0);
                Ok(Response::new(ValidUserRsp {
                    correct: true,
                    vid: vid as u64,
                }))
            }
        } else {
            Err(Status::not_found("user/email not found"))
        }
    }

    async fn get_user(
        &self,
        request: Request<GetUserReq>,
    ) -> Result<Response<GetUserRsp>, Status> {
        let req = request.into_inner();
        let res: Option<table::User> = match sqlx::query_as::<_, table::User>(
            "SELECT * from user_tbl where vid=$1")
            .bind(&(req.vid as i64))
            .fetch_optional(&self.pool)
            .await
        {
            Ok(v) => v,
            Err(err) => {
                error!("execute sql failed when select. err {}", err);
                return Err(Status::unavailable("execute sql failed"));
            }
        };
        if let Some(user) = res {
            Ok(Response::new(GetUserRsp {
                userinfo: Some(user::UserInfo {
                    vid: user.vid as u64,
                    username: user.username,
                    nickname: user.nickname,
                    avatar: user.avatar,
                    email: user.email,
                })
            }))
        }
        else {
            Err(Status::not_found("user not found"))
        }
    }

    async fn update_user(
        &self,
        request: Request<UpdateUserReq>,
    ) -> Result<Response<UpdateUserRsp>, Status> {
        let req = request.into_inner();
        let userinfo = req.userinfo.unwrap_or_default();
        let res: Option<table::User> = match sqlx::query_as::<_, table::User>(
            "SELECT * from user_tbl where vid=$1")
            .bind(&(userinfo.vid as i64))
            .fetch_optional(&self.pool)
            .await
        {
            Ok(v) => v,
            Err(err) => {
                error!("execute sql failed when select. err {}", err);
                return Err(Status::unavailable("execute sql failed"));
            }
        };
        if let Some(user) = res {
            let avatar = match userinfo.avatar.len() {
                0 => user.avatar,
                _ => userinfo.avatar,
            };
            let nickname = match userinfo.nickname.len() {
                0 => user.nickname,
                _ => userinfo.nickname,
            };
            if let Err(err) = sqlx::query("UPDATE user_tbl set avatar=$1, set nickname=$2 where vid=$3")
                .bind(&avatar)
                .bind(&nickname)
                .bind(&user.vid)
                .execute(&self.pool).await {
                    error!("execute sql failed when select. err {}", err);
                    return Err(Status::unavailable("execute sql failed"));
            }
        }
        Ok(Response::new(UpdateUserRsp {}))
    }

    async fn update_password(
        &self,
        request: Request<UpdatePasswordReq>,
    ) -> Result<Response<UpdatePasswordRsp>, Status> {
        let req = request.into_inner();
        let res: Option<table::User> = match sqlx::query_as::<_, table::User>(
            "SELECT * from user_tbl where vid=$1")
            .bind(&(req.vid as i64))
            .fetch_optional(&self.pool)
            .await
        {
            Ok(v) => v,
            Err(err) => {
                error!("execute sql failed when select. err {}", err);
                return Err(Status::unavailable("execute sql failed"));
            }
        };
        if let Some(user) = res {
            if make_password_crypto(&req.old_password, &user.salt) != user.password {
                return Err(Status::internal("password is not match"));
            }

            let new_salt = random_salt();
            let pwd_crypto = make_password_crypto(&req.password, &new_salt);
            if let Err(err) = sqlx::query("UPDATE user_tbl set password=$1, set salt=$2 where vid=$3")
                .bind(&pwd_crypto)
                .bind(&new_salt)
                .bind(&user.vid)
                .execute(&self.pool).await {
                    error!("execute sql failed when select. err {}", err);
                    return Err(Status::unavailable("execute sql failed"));
            }
            Ok(Response::new(UpdatePasswordRsp {}))
        }
        else {
            Err(Status::not_found("user not found"))
        }
    }
}

pub async fn get(database_url: &str, micro_service: Arc<MicroService>) -> UserSvrServer<UserSvrImpl> {
    let connections: u32 = 10;
    let pool = match PgPoolOptions::new()
        .max_connections(connections)
        .connect_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await {
            Ok(v) => v,
            Err(err) => {
                error!("connect database err {}", err);
                std::process::exit(-1);
            }
        };

    return UserSvrServer::new(UserSvrImpl {
        pool: pool,
        micro_service,
    });
}
