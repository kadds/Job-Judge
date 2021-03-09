macro_rules! check_rpc {
    ($e: expr) => {
        match $e {
            Ok(res) => res.into_inner(),
            Err(status) => return crate::util::build_fail_response(status),
        }
    };
}
use actix_web::web;
type Context = web::Data<std::sync::Arc<crate::AppData>>;
pub mod comm;
pub mod user;
