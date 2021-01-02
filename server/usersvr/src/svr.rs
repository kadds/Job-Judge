use micro_service::{log, service::MicroService};
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use sqlx::postgres::{PgPoolOptions, PgPool};
use sqlx::Row;
use tonic::{Request, Response, Status};

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

#[tonic::async_trait]
impl UserSvr for UserSvrImpl {
    async fn create_user(
        &self,
        request: Request<CreateUserReq>,
    ) -> Result<Response<CreateUserRsp>, Status> {
        let req = request.into_inner();
        let salt = "123";
        let pwd = req.password;

        let id: i32 = match sqlx::query("INSERT INTO user_tbl 
        (username, password, salt, nickname) VALUES 
        ($1, $2, $3, $4)
        RETURNING id")
            .bind(&req.username)
            .bind(&pwd)
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
            "SELECT password, salt from user_tbl where username=$1")
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

        return if let Some(res) = res {
            let pwd: String = res.get(0);
            if pwd != req.password {
                Ok(Response::new(ValidUserRsp {
                    correct: false
                }))
            } else {
                Ok(Response::new(ValidUserRsp {
                    correct: true
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
        Err(Status::unavailable(""))
    }

    async fn update_user(
        &self,
        request: Request<UpdateUserReq>,
    ) -> Result<Response<UpdateUserRsp>, Status> {
        Err(Status::unavailable(""))

        //Ok(Response::new(UpdateUserResult {}))
    }
}
/*
"
CREATE TABLE user_tbl (
    id SERIAL PRIMARY KEY,
    username VARCHAR NOT NULL,
    password VARCHAR NOT NULL,
    salt VARCHAR NOT NULL,
    nickname VARCHAR NOT NULL,
    email VARCHAR,
    phone VARCHAR,
    avatar VARCHAR
)
"
*/

pub async fn get(database_url: &str, micro_service: Arc<MicroService>) -> UserSvrServer<UserSvrImpl> {
    let connections: u32 = 10;
    let pool = match PgPoolOptions::new()
        .max_connections(connections)
        .connect_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await {
            Ok(v) => v,
            Err(err) => {
                error!("prepare/connect database err {}", err);
                std::process::exit(-1);
            }
        };

    return UserSvrServer::new(UserSvrImpl {
        pool: pool,
        micro_service,
    });
}
