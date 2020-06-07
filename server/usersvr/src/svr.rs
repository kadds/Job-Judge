use dotenv::dotenv;
use log::{debug, error, info, trace, warn};
use rpc::user_svr_server::{UserSvr, UserSvrServer};
use rpc::*;
use tokio_postgres::{Client, Connection, Error, Statement};
//use schema::user_tbl::dsl::*;
use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::time::Duration;
use tokio::time::timeout;
use tonic::{Request, Response, Status};

pub mod rpc {
    tonic::include_proto!("rpc");
}
pub mod user {
    tonic::include_proto!("user");
}

pub struct UserSvrImpl {
    client: Client,
    statements: Vec<Statement>,
}

#[tonic::async_trait]
impl UserSvr for UserSvrImpl {
    async fn create_user(
        &self,
        request: Request<CreateUserRequest>,
    ) -> Result<Response<CreateUserResult>, Status> {
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

        Ok(Response::new(CreateUserResult { uid: id as u64 }))
    }

    async fn valid_user(
        &self,
        request: Request<ValidUserRequest>,
    ) -> Result<Response<ValidUserResult>, Status> {
        Err(Status::unavailable(""))
        //Ok(Response::new(ValidUserResult {}))
    }

    async fn get_user(
        &self,
        request: Request<GetUserRequest>,
    ) -> Result<Response<GetUserResult>, Status> {
        Err(Status::unavailable(""))
    }
    async fn update_user(
        &self,
        request: Request<UpdateUserRequest>,
    ) -> Result<Response<UpdateUserResult>, Status> {
        Err(Status::unavailable(""))

        //Ok(Response::new(UpdateUserResult {}))
    }
    async fn has_user(
        &self,
        request: Request<HasUserRequest>,
    ) -> Result<Response<HasUserResult>, Status> {
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
    avatar VARCHAR
)
"
*/

async fn prepare_all() -> Result<(Client, Vec<Statement>), Error> {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let (client, connection) =
        tokio_postgres::connect(&database_url, tokio_postgres::NoTls).await?;

    connection.await?;

    let mut s: Vec<Statement> = vec![];
    let all_sql = [
        "INSERT INTO user_tbl 
        (username, password, salt, nickname) VALUES 
        ($1, $2, $3, $4)
        RETURNING id",
        "",
    ];
    let arr = all_sql.iter().map(|&v| client.prepare(v));

    for it in futures::future::try_join_all(arr).await? {
        s.push(it);
    }

    Ok((client, s))
}

pub async fn get() -> UserSvrServer<UserSvrImpl> {
    let (client, s) = match prepare_all().await {
        Ok(v) => v,
        Err(err) => {
            error!("prepare/connect database err {}", err);
            std::process::exit(-1);
        }
    };

    return UserSvrServer::new(UserSvrImpl {
        client: client,
        statements: s,
    });
}
