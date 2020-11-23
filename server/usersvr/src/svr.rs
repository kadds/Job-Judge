use micro_service::{log, service::MicroService};
use std::env;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use tokio_postgres::{Client, Connection, Error, Row, Statement};
use tonic::{Request, Response, Status};

pub mod user {
    pub mod rpc {
        tonic::include_proto!("user.rpc");
    }
    tonic::include_proto!("user");
}

use user::rpc::user_svr_server::{UserSvr, UserSvrServer};
use user::rpc::*;
use user::*;

pub struct UserSvrImpl {
    client: Client,
    statements: Vec<Statement>,
    micro_service: Arc<MicroService>,
}

#[tonic::async_trait]
impl UserSvr for UserSvrImpl {
    async fn create_user(
        &self,
        request: Request<CreateUserReq>,
    ) -> Result<Response<CreateUserRsp>, Status> {
        let req = request.into_inner();
        let userinfo = match req.userinfo {
            Some(v) => v,
            None => {
                return Err(Status::invalid_argument("get user info empty"));
            }
        };

        let id: i32 = match self
            .client
            .query_one(
                &self.statements[0],
                &[
                    &userinfo.username,
                    &userinfo.password,
                    &userinfo.salt,
                    &userinfo.nickname,
                ],
            )
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
        let mut ok = false;
        let mut exist = false;
        let res: Option<Row> = match self
            .client
            .query_opt(&self.statements[1], &[&req.username])
            .await
        {
            Ok(v) => v,
            Err(err) => {
                error!("execute sql failed when select. err {}", err);
                return Err(Status::unavailable("execute sql failed"));
            }
        };

        if let Some(res) = res {
            exist = true;
            let pwd: String = res.get(0);
            if pwd != req.password {
                ok = false;
            } else {
                ok = true;
            }
        } else {
            ok = false;
            exist = false;
        }

        Ok(Response::new(ValidUserRsp {
            ok,
            is_exist: exist,
        }))
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
    async fn has_user(
        &self,
        request: Request<HasUserReq>,
    ) -> Result<Response<HasUserRsp>, Status> {
        Err(Status::unavailable(""))
        //Ok(Response::new(HasUserResult {}))
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

async fn prepare_all(database_url: &str) -> Result<(Client, Vec<Statement>), Error> {
    let (client, connection) =
        tokio_postgres::connect(database_url, tokio_postgres::NoTls).await?;

    connection.await?;

    let mut s: Vec<Statement> = vec![];
    let all_sql = [
        "INSERT INTO user_tbl 
        (username, password, salt, nickname) VALUES 
        ($1, $2, $3, $4)
        RETURNING id",

        "SELECT password from user_tbl where username=$1",
    ];
    let arr = all_sql.iter().map(|&v| client.prepare(v));

    for it in futures::future::try_join_all(arr).await? {
        s.push(it);
    }

    Ok((client, s))
}

pub async fn get(database_url: &str, micro_service: Arc<MicroService>) -> UserSvrServer<UserSvrImpl> {
    let (client, s) = match prepare_all(database_url).await {
        Ok(v) => v,
        Err(err) => {
            error!("prepare/connect database err {}", err);
            std::process::exit(-1);
        }
    };

    return UserSvrServer::new(UserSvrImpl {
        client: client,
        statements: s,
        micro_service,
    });
}
