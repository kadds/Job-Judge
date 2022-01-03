use crate::{token, AppData};
use actix_web::{
    body::{EitherBody, MessageBody},
    dev::Service,
    dev::ServiceRequest,
    dev::ServiceResponse,
    dev::Transform,
    error, web, Error,
};
use futures::Future;
use futures::{
    future::{ok, Ready},
    FutureExt,
};
use std::pin::Pin;
use std::{
    sync::Arc,
    task::{Context, Poll},
};

pub struct Auth {}

impl Auth {
    pub fn new() -> Auth {
        Auth {}
    }
}

impl<S, B> Transform<S, ServiceRequest> for Auth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody,
    S::Future: 'static,
{
    type Error = Error;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;
    type InitError = ();
    type Response = ServiceResponse<EitherBody<B>>;
    type Transform = AuthMiddleware<S>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AuthMiddleware {
            service: service.into(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct AuthMiddleware<S> {
    service: Arc<S>,
}

impl<S, B> Service<ServiceRequest> for AuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody,
    S::Future: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        async move {
            let uri = req.uri();
            let data = &req.app_data::<web::Data<AppData>>().unwrap();
            let need_token = uri != "/api/user/login" && data.config.comm.username.is_some();
            let token = req
                .headers()
                .get("token")
                .map(|v| v.to_str().unwrap_or("").to_owned())
                .unwrap_or_default();
            if !need_token || token::is_valid(&token).await {
                service.call(req).await.map(|res| res.map_into_left_body())
            } else {
                log::error!("authorize fail");
                Ok(req
                    .error_response(error::ErrorUnauthorized("need authorized"))
                    .map_into_right_body())
            }
        }
        .boxed_local()
    }
}
