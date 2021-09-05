use actix_web::{
    dev::Service,
    dev::ServiceRequest,
    dev::ServiceResponse,
    dev::{MessageBody, Transform},
    http::{HeaderName, HeaderValue},
    Error,
};
use futures::future::{ok, Ready};
use futures::Future;
use std::task::{Context, Poll};
use std::{pin::Pin, time::Instant};

pub struct RequestMetrics {}

impl RequestMetrics {
    pub fn new() -> Self {
        Self {}
    }
}

impl<S, B> Transform<S, ServiceRequest> for RequestMetrics
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    B: MessageBody,
    S::Future: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = RequestMetricsMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(RequestMetricsMiddleware { service })
    }
}

pub struct RequestMetricsMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for RequestMetricsMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    B: MessageBody,
    S::Future: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let t = Instant::now();

        let fut = self.service.call(req);
        Box::pin(async move {
            let mut res = fut.await?;
            let end = Instant::now();
            let cost = format!("{}ms", (end - t).as_millis());
            let cost = HeaderValue::from_maybe_shared(cost).unwrap();
            res.response_mut()
                .headers_mut()
                .append(HeaderName::from_static("cost"), cost);
            Ok(res)
        })
    }
}
