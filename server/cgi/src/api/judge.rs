use actix_web::{
    connect, delete, get, head, options, patch, post, put, trace, web::Json, web::Path,
    HttpResponse, Responder,
};

use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct CommitSource {
    lang_id: u32,
    problem_id: u32,
    source_code: String,
}

#[derive(Serialize)]
pub struct CommitResult {
    id: u64,
}

#[post("/commit")]
pub async fn commit(source: Json<CommitSource>) -> impl Responder {
    HttpResponse::Ok().json(CommitResult { id: 0 })
}

#[derive(Serialize)]
struct Lang {
    name: String,
    lid: u32,
}

#[derive(Serialize)]
struct LangList {
    langs: Vec<Lang>,
}

#[get("/langs")]
pub async fn lang_list() -> impl Responder {
    HttpResponse::Ok().json(LangList { langs: vec![] })
}
