use log::{debug, info, trace, warn};
use rpc::compilation_svr_server::{CompilationSvr, CompilationSvrServer};
use rpc::{CompilationRequest, CompilationResult};
use std::fs::File;
use std::io::{Read, Write};
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;
use tonic::{Request, Response, Status};

pub mod rpc {
    tonic::include_proto!("rpc");
}
pub mod runner {
    tonic::include_proto!("runner");
}

use runner::Lang;

fn get_lang_shell<'a>(lang: i32) -> Option<&'a str> {
    match lang {
        x if x == Lang::C as i32 => Some("c.sh"),
        x if x == Lang::Cpp as i32 => Some("cpp.sh"),
        x if x == Lang::Java as i32 => Some("java.sh"),
        _ => None,
    }
}

fn get_lang_source_file<'a>(lang: i32) -> Option<&'a str> {
    match lang {
        x if x == Lang::C as i32 => Some("main.c"),
        x if x == Lang::Cpp as i32 => Some("main.cpp"),
        x if x == Lang::Java as i32 => Some("Main.java"),
        _ => None,
    }
}

fn get_lang_output_file<'a>(lang: i32) -> Option<&'a str> {
    match lang {
        x if x == Lang::C as i32 => Some("main"),
        x if x == Lang::Cpp as i32 => Some("main"),
        x if x == Lang::Java as i32 => Some("Main.class"),
        _ => None,
    }
}

#[derive(Debug, Default)]
pub struct CompilationSvrImpl {}

#[tonic::async_trait]
impl CompilationSvr for CompilationSvrImpl {
    async fn compile(
        &self,
        request: Request<CompilationRequest>,
    ) -> Result<Response<CompilationResult>, Status> {
        let req = request.into_inner();
        let start = chrono::Utc::now();

        let shell = match get_lang_shell(req.lang) {
            Some(x) => x,
            None => {
                warn!("lang is unknown. request {:?}", req);
                return Ok(Response::new(CompilationResult {
                    cost: 0,
                    result_bin: req.source_code.as_bytes().to_vec(),
                }));
            }
        };

        let output = get_lang_output_file(req.lang).unwrap();
        let input = get_lang_source_file(req.lang).unwrap();

        if File::create(input)
            .and_then(|mut f| f.write_all(req.source_code.as_bytes()).map(|_| f))
            .and_then(|mut f| f.flush())
            .is_err()
        {
            warn!("write source code failed. request {:?}", req);
            return Err(Status::internal("write source code failed"));
        }

        let cmd_timeout = timeout(
            Duration::from_secs(10),
            Command::new("./compilation/".to_owned() + shell)
                .arg(&input)
                .arg(&output)
                .arg(req.extern_flags)
                .output(),
        )
        .await;

        let cmd_result = match cmd_timeout {
            Ok(v) => v,
            Err(_) => {
                return Err(Status::resource_exhausted("timeout"));
            }
        };

        let cmd = match cmd_result {
            Ok(v) => v,
            Err(err) => {
                return Err(Status::resource_exhausted(
                    "execute command failed. ".to_owned() + err.to_string().as_str(),
                ))
            }
        };

        let output_obj = cmd;

        if !output_obj.status.success() {
            let err_str =
                String::from_utf8(output_obj.stderr).unwrap_or("unknown error".to_string());
            return Err(Status::internal(err_str));
        }
        let end = chrono::Utc::now();
        let delta = end.timestamp_millis() - start.timestamp_millis();
        let mut buf = vec![];

        if File::open(output)
            .and_then(|mut f| f.read_to_end(&mut buf))
            .is_err()
        {
            return Err(Status::unavailable("compilation error"));
        }

        Ok(Response::new(CompilationResult {
            cost: delta as u64,
            result_bin: buf,
        }))
    }
}

pub fn get() -> CompilationSvrServer<CompilationSvrImpl> {
    return CompilationSvrServer::new(CompilationSvrImpl::default());
}

#[cfg(test)]
#[test]
fn test() {
    let source_code = r#"
#include <iostream>
int main()
{
    int num;
    std::cin >> num;
    std::cout << num + 1;
    return 0;
}
"#;
    liblog::init_test_logger().unwrap();

    let mut runtime = tokio::runtime::Builder::new()
        .max_threads(1)
        .core_threads(1)
        .enable_all()
        .build()
        .unwrap();
    let svr = CompilationSvrImpl::default();
    let req = Request::new(CompilationRequest {
        lang: 1,
        lang_version: "11".to_owned(),
        source_code: String::from(source_code),
        extern_flags: "".to_owned(),
    });
    runtime.block_on(async {
        let async_ret = svr.compile(req).await;
        let respond = async_ret.unwrap().into_inner();
        assert_ne!(0, respond.cost);
        assert!(0 < respond.result_bin.len());
    })
}
