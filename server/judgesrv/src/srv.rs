use log::{debug, info, trace, warn};
use rpc::judge_srv_server::{JudgeSrv, JudgeSrvServer};
use rpc::{JudgeRequest, JudgeResult, JudgeStatistics, TestCase};
use std::fmt;
use std::fs::File;
use std::io::Write;
use std::process::Stdio;
use std::thread;

use std::os::unix::io::{AsRawFd, FromRawFd};
use tokio::io::{self, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::prelude::*;
use tokio::process::Command;
use tonic::{Request, Response, Status};

pub mod rpc {
    tonic::include_proto!("rpc");
}
pub mod runner {
    tonic::include_proto!("runner");
}
use runner::Lang;
use runner::ResultType;

unsafe fn set_limit(data_mem: u64, cpu_time: u64) {
    let mut r = libc::rlimit64 {
        rlim_cur: 0,
        rlim_max: 0,
    };
    r.rlim_cur = data_mem;

    libc::setrlimit64(libc::RLIMIT_AS, &r);
    r.rlim_cur = cpu_time / 1000;
    libc::setrlimit64(libc::RLIMIT_CPU, &r);
    r.rlim_cur = 1;
    libc::setrlimit64(libc::RLIMIT_NPROC, &r);

    r.rlim_cur = data_mem;
    libc::setrlimit64(libc::RLIMIT_DATA, &r);
}

unsafe fn rusage_new() -> libc::rusage {
    libc::rusage {
        ru_utime: libc::timeval {
            tv_sec: 0,
            tv_usec: 0,
        },
        ru_stime: libc::timeval {
            tv_sec: 0,
            tv_usec: 0,
        },
        ru_maxrss: 0,
        ru_ixrss: 0,
        ru_idrss: 0,
        ru_isrss: 0,
        ru_minflt: 0,
        ru_majflt: 0,
        ru_nswap: 0,
        ru_inblock: 0,
        ru_oublock: 0,
        ru_msgsnd: 0,
        ru_msgrcv: 0,
        ru_nsignals: 0,
        ru_nvcsw: 0,
        ru_nivcsw: 0,
    }
}

unsafe fn bind_stdio(stdio_fd: ([i32; 2], [i32; 2], [i32; 2])) -> Option<()> {
    if libc::dup2(0, stdio_fd.0[0]) != 0 {
        return None;
    }
    if libc::dup2(1, stdio_fd.1[1]) != 0 {
        return None;
    }
    if libc::dup2(2, stdio_fd.2[1]) != 0 {
        return None;
    }
    Some(())
}

unsafe fn create_stdio() -> Option<([i32; 2], [i32; 2], [i32; 2])> {
    let mut cins: [libc::c_int; 2] = [0, 0];
    let mut cous: [libc::c_int; 2] = [0, 0];
    let mut cerrs: [libc::c_int; 2] = [0, 0];

    if libc::pipe(cins.as_mut_ptr()) != 0 {
        return None;
    }
    if libc::pipe(cous.as_mut_ptr()) != 0 {
        return None;
    }
    if libc::pipe(cerrs.as_mut_ptr()) != 0 {
        return None;
    }

    Some((cins, cous, cerrs))
}

fn get_lang_filename<'a>(lang: i32) -> Option<&'a str> {
    match lang {
        x if x == Lang::C as i32 => Some("main"),
        x if x == Lang::Cpp as i32 => Some("main"),
        x if x == Lang::Java as i32 => Some("Main.class"),
        x if x == Lang::Python as i32 => Some("main.py"),
        x if x == Lang::Javascript as i32 => Some("main.js"),
        _ => None,
    }
}

fn get_lang_exec_cmd<'a, 'b>(lang: i32) -> Option<(&'a str, &'b str)> {
    match lang {
        x if x == Lang::C as i32 => Some(("./main", "")),
        x if x == Lang::Cpp as i32 => Some(("./main", "")),
        x if x == Lang::Java as i32 => Some(("java", "Main.class")),
        x if x == Lang::Python as i32 => Some(("python", "main.py")),
        x if x == Lang::Javascript as i32 => Some(("node", "main.js")),
        _ => None,
    }
}

fn do_thread(stdin: i32, input: String) {
    unsafe {
        let mut writer = File::from_raw_fd(stdin);
        if let Err(_) = writer.write_all(input.as_bytes()) {
            return;
        }
    }
}

fn build_avg(stat: &mut JudgeStatistics, n: usize) {
    let n = n + 1;
    stat.vir_time_cost = stat.vir_time_cost / n as u64;
    stat.real_time_cost = stat.real_time_cost / n as u64;
    stat.mm_cost = stat.mm_cost / n as u64;
}

impl fmt::Display for JudgeRequest {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let stat = self.limit_stat.as_ref().unwrap();
        write!(f, "lang {}，lang_version {}, extern_flags {}, limit_real_time {}ms, limit_vir_time {}ms, limit_mm {}Mib, test_case {:?}", 
        self.lang, self.lang_version, self.extern_flags, stat.real_time_cost, stat.vir_time_cost,
            stat.mm_cost / 1024 / 1024, self.test_case)
    }
}

#[derive(Debug, Default)]
pub struct JudgeSrvImpl {}

#[tonic::async_trait]
impl JudgeSrv for JudgeSrvImpl {
    async fn judge(&self, request: Request<JudgeRequest>) -> Result<Response<JudgeResult>, Status> {
        let req = request.into_inner();
        trace!("request {}", req);

        let filename = match get_lang_filename(req.lang) {
            Some(x) => x,
            None => {
                warn!("lang is unknown.");
                return Err(Status::failed_precondition("unknown lang"));
            }
        };

        if let Err(err) = File::create(filename)
            .and_then(|mut f| f.write_all(&req.result_bin).map(|_| f))
            .and_then(|mut f| f.flush())
        {
            warn!(
                "write bin to file {} failed. because {}",
                filename,
                err.to_string()
            );
            return Err(Status::unavailable(err.to_string()));
        };

        let (cmd, arg) = match get_lang_exec_cmd(req.lang) {
            Some(x) => x,
            None => {
                warn!("lang is unknown.");
                return Err(Status::failed_precondition("unknown lang"));
            }
        };

        let mut i: usize = 0;
        let stat = req.limit_stat.unwrap();
        let limit_vir_time = stat.vir_time_cost;
        let limit_real_time = stat.real_time_cost;
        let limit_mm = stat.mm_cost;

        let mut min_stat = JudgeStatistics {
            vir_time_cost: u64::MAX,
            real_time_cost: u64::MAX,
            mm_cost: u64::MAX,
        };
        let mut avg_stat = JudgeStatistics {
            vir_time_cost: 0,
            real_time_cost: 0,
            mm_cost: 0,
        };
        let mut max_stat = JudgeStatistics {
            vir_time_cost: 0,
            real_time_cost: 0,
            mm_cost: 0,
        };

        for test_case in req.test_case {
            unsafe {
                let tc_start = chrono::Utc::now().timestamp_millis();

                let child = match Command::new(cmd)
                    .arg(arg)
                    .arg(req.extern_flags.to_owned())
                    .kill_on_drop(true)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .pre_exec(move || {
                        set_limit(limit_mm, limit_vir_time);
                        Ok(())
                    })
                    .spawn()
                {
                    Ok(x) => x,
                    Err(err) => {
                        warn!(
                            "create process failed, cmd: {} {}, because {}",
                            cmd,
                            arg,
                            err.to_string(),
                        );
                        return Err(Status::internal(err.to_string()));
                    }
                };
                let pid = child.id();

                let stdin = child.stdin.unwrap();
                let stdout = child.stdout.unwrap();
                let mut reader = BufReader::new(stdout);
                let mut buf = vec![];
                let stdin_fd = stdin.as_raw_fd();

                let input = test_case.input;
                let output = test_case.output;

                let thd = thread::spawn(move || do_thread(stdin_fd, input));

                reader.read_to_end(&mut buf).await;

                if thd.join().is_err() {
                    warn!("join thread failed");
                    return Err(Status::internal("join thread failed"));
                }
                let mut state: libc::c_int = 0;
                let mut rusage = rusage_new();

                libc::wait4(pid as i32, &mut state, 0, &mut rusage);

                let cur_real_time = (chrono::Utc::now().timestamp_millis() - tc_start) as u64;
                let cur_vir_time =
                    (rusage.ru_utime.tv_sec * 1000 + rusage.ru_utime.tv_usec / 1000) as u64;
                let cur_mm = rusage.ru_maxrss as u64 * 1024;

                max_stat.real_time_cost = std::cmp::max(max_stat.real_time_cost, cur_real_time);
                max_stat.vir_time_cost = std::cmp::max(max_stat.vir_time_cost, cur_vir_time);
                max_stat.mm_cost = std::cmp::max(max_stat.mm_cost, cur_mm);

                min_stat.real_time_cost = std::cmp::min(min_stat.real_time_cost, cur_real_time);
                min_stat.vir_time_cost = std::cmp::min(min_stat.vir_time_cost, cur_vir_time);
                min_stat.mm_cost = std::cmp::min(min_stat.mm_cost, cur_mm);

                avg_stat.real_time_cost += cur_real_time;
                avg_stat.vir_time_cost += cur_vir_time;
                avg_stat.mm_cost += cur_mm;

                if !libc::WIFEXITED(state) {
                    build_avg(&mut avg_stat, i);
                    warn!("core dumped with exit code {}", state);

                    return Ok(Response::new(JudgeResult {
                        max_stat: Some(max_stat),
                        min_stat: Some(min_stat),
                        avg_stat: Some(avg_stat),
                        error_str: "Process core dumped, check your program.".to_owned(),
                        r#type: ResultType::RuntimeError as i32,
                        error_test_case: i as u64,
                    }));
                }

                match String::from_utf8(buf) {
                    Err(v) => {
                        build_avg(&mut avg_stat, i);
                        warn!("output string is invalid");

                        return Ok(Response::new(JudgeResult {
                            max_stat: Some(max_stat),
                            min_stat: Some(min_stat),
                            avg_stat: Some(avg_stat),
                            error_str: "Invalid output character, please print with UTF-8 format"
                                .to_owned(),
                            r#type: ResultType::WrongAnswer as i32,
                            error_test_case: i as u64,
                        }));
                    }
                    Ok(s) => {
                        if s != output {
                            build_avg(&mut avg_stat, i);
                            debug!("'{}' we get is not like expect '{}'", s, output);

                            return Ok(Response::new(JudgeResult {
                                max_stat: Some(max_stat),
                                min_stat: Some(min_stat),
                                avg_stat: Some(avg_stat),
                                error_str: s,
                                r#type: ResultType::WrongAnswer as i32,
                                error_test_case: i as u64,
                            }));
                        }
                    }
                }
                i += 1;
            }
        }
        build_avg(&mut avg_stat, i);
        return Ok(Response::new(JudgeResult {
            max_stat: Some(max_stat),
            min_stat: Some(min_stat),
            avg_stat: Some(avg_stat),
            error_str: "".to_owned(),
            r#type: ResultType::AllCorrect as i32,
            error_test_case: 0,
        }));
    }
}

pub fn get() -> JudgeSrvServer<JudgeSrvImpl> {
    return JudgeSrvServer::new(JudgeSrvImpl::default());
}

#[test]
fn test() {
    let source_code = r#"
import time
i = int(input())
time.sleep(1)
print(i + 1)"#;
    liblog::init_test_logger().unwrap();

    let mut runtime = tokio::runtime::Builder::new()
        .max_threads(1)
        .core_threads(1)
        .enable_all()
        .build()
        .unwrap();
    let srv = JudgeSrvImpl::default();

    let req = Request::new(JudgeRequest {
        lang: Lang::Python as i32,
        lang_version: "3".to_owned(),
        result_bin: String::from(source_code).as_bytes().to_vec(),
        extern_flags: "".to_owned(),
        test_case: vec![TestCase {
            input: "1".to_owned(),
            output: "2\n".to_owned(),
        }],
        limit_stat: Some(JudgeStatistics {
            real_time_cost: 2000,
            vir_time_cost: 2000,
            mm_cost: 10 * 1024 * 1024,
        }),
    });

    runtime.block_on(async {
        let async_ret = srv.judge(req).await;
        let respond = async_ret.unwrap().into_inner();
        if respond.r#type != 0 {
            assert_eq!("here is output str", respond.error_str);
        }
    })
}
