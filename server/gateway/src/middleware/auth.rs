use std::pin::Pin;
use std::task::{Context, Poll};

use crate::{util::is_valid_token, AppData};
use actix_web::{
    dev::Service,
    dev::ServiceRequest,
    dev::ServiceResponse,
    dev::{MessageBody, Transform},
    web,
    error, Error,
};
use futures::future::{ok, Ready};
use futures::Future;

pub struct Auth {}

impl Auth {
    pub fn new() -> Auth {
        Auth {}
    }
}

impl<S, B> Transform<S, ServiceRequest> for Auth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    B: MessageBody,
    S::Future: 'static,
{
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

impl<S, B> Service<ServiceRequest> for AuthMiddleware<S>
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
        let uri = req.uri();
        let server = req.app_data::<web::Data<AppData>>().unwrap().server.clone();
        let need_token = uri != "/user/login" && uri != "/user/register";
        let token = req
            .headers()
            .get("TOKEN")
            .map(|v| v.to_str().unwrap_or("").to_owned());

        let fut = self.service.call(req);
        Box::pin(async move {
            if need_token {
                if let Some(token) = token {
                    if !is_valid_token(server, token).await {
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
