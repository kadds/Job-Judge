use std::pin::Pin;
use std::task::{Context, Poll};

use super::super::{token, AppData};
use actix_web::{
    dev::Service, dev::ServiceRequest, dev::ServiceResponse, dev::Transform, error, Error,
};
use futures::future::{ok, Ready};
use futures::Future;
use std::sync::Arc;

pub struct Auth {}

impl Auth {
    pub fn new() -> Auth {
        Auth {}
    }
}

impl<S, B> Transform<S> for Auth
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AuthMiddleware { service })
    }
}

pub struct AuthMiddleware<S> {
    service: S,
}

impl<S, B> Service for AuthMiddleware<S>
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
        let uri = req.uri();
        let data = &req.app_data::<Arc<AppData>>().unwrap();
        let need_token = uri != "/user/login" && !data.config.user.no_verify;
        let token = req
            .headers()
            .get("TOKEN")
            .map(|v| v.to_str().unwrap_or("").to_owned());

        let fut = self.service.call(req);
        Box::pin(async move {
            if need_token {
                if let Some(token) = token {
                    if !token::is_valid(&token).await {
                        return Err(error::ErrorUnauthorized("authorize fail"));
                    }
                } else {
                    return Err(error::ErrorUnauthorized("need authorized"));
                }
            }
            let res = fut.await?;
            Ok(res)
        })
    }
}
