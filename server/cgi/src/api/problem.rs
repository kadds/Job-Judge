use actix_web::{
    connect, delete, get, head, options, patch, post, put, trace, web::Json, web::Path,
    HttpResponse, Responder,
};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct TestCase {
    input: String,
    output: String,
}

#[derive(Serialize, Deserialize)]
pub struct Problem {
    id: u64,
    title: String,
    md_text: String,
    test_case: Vec<TestCase>,
}

#[derive(Serialize, Deserialize)]
pub struct ProblemInfo {
    id: u64,
    title: String,
    md_text: String,
}

#[get("/{id}")]
pub async fn problem(path: Path<(u64,)>) -> impl Responder {
    HttpResponse::Ok().json(ProblemInfo {
        id: path.0,
        title: "test".to_owned(),
        md_text: "test".to_owned(),
    })
}

#[post("")]
pub async fn create_problem(info: Json<Problem>) -> impl Responder {
    HttpResponse::Ok()
}

#[put("")]
pub async fn update_problem(info: Json<Problem>) -> impl Responder {
    HttpResponse::Ok()
}

#[put("")]
pub async fn update_test_case(info: Json<Problem>) -> impl Responder {
    HttpResponse::Ok()
}
