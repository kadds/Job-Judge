use std::pin::Pin;
use std::task::{Context, Poll};

use actix_web::{
    dev::Body, dev::BodySize, dev::ResponseBody, dev::Service, dev::ServiceRequest,
    dev::ServiceResponse, dev::Transform, Error,
};
use futures::future::{ok, Ready};
use futures::Future;
use micro_service::{log, service::MicroService, util};
use std::sync::Arc;

pub struct Logger {
    micro_service: Arc<MicroService>,
}

impl Logger {
    pub fn new(micro_service: Arc<MicroService>) -> Logger {
        Logger { micro_service }
    }
}

impl<S, B> Transform<S> for Logger
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = LoggerMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(LoggerMiddleware {
            service,
            micro_service: self.micro_service.clone(),
        })
    }
}

pub struct LoggerMiddleware<S> {
    service: S,
    micro_service: Arc<MicroService>,
}

impl<S, B> Service for LoggerMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, req: ServiceRequest) -> Self::Future {
        let ts = util::current_ts();
        let vid = 0;
        let nid = util::gen_nid();
        let tid = util::gen_tid();

        let fut = self.service.call(req);
        let server_name = self.micro_service.service_name().clone();

        Box::pin(log::make_context(
            vid,
            tid,
            nid,
            0,
            server_name,
            async move {
                let res = fut.await?;
                let req = res.request();
                let method = req.method();
                let uri = req.uri();
                // let path = req.path();
                let host = req
                    .peer_addr()
                    .map_or_else(|| "0.0.0.0:0".to_string(), |v| v.to_string());
                let cost = util::current_ts() - ts;
                let status = res.status().as_u16();
                let len = match res.response().body() {
                    ResponseBody::Body(_) => 0,
                    ResponseBody::Other(b) => match b {
                        Body::Bytes(bytes) => bytes.len() as u64,
                        Body::Message(mb) => match mb.size() {
                            BodySize::Sized(s) => s,
                            _ => 0,
                        },
                        _ => 0,
                    },
                };
                click_log!(ts, cost, method, uri, host, status, len);
                Ok(res)
            },
        ))
    }
}
