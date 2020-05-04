use actix_web::{
    connect, delete, get, head, options, patch, post, put, trace, web::Json, web::Path,
    HttpResponse, Responder,
};

use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct RunSource {
    lang_id: u32,
    source_code: String,
    inputs: Vec<String>,
}

#[derive(Serialize)]
pub struct RunResult {
    id: u64,
}

#[post("")]
pub async fn run_source(source: Json<RunSource>) -> impl Responder {
    HttpResponse::Ok().json(RunResult { id: 0 })
}
