use super::super::{token, AppData};
use actix_web::{
    body::AnyBody,
    dev::Service,
    dev::ServiceRequest,
    dev::ServiceResponse,
    dev::{MessageBody, Transform},
    error, web, Error,
};
use futures::future::{ok, Ready};
use futures::Future;
use std::task::{Context, Poll};
use std::{pin::Pin, rc::Rc};

pub struct Auth {}

impl Auth {
    pub fn new() -> Auth {
        Auth {}
    }
}

impl<S, B> Transform<S, ServiceRequest> for Auth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody + 'static,
    S::Future: 'static,
    B::Error: std::error::Error,
{
    type Error = Error;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;
    type InitError = ();
    type Response = ServiceResponse;
    type Transform = AuthMiddleware<S>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AuthMiddleware {
            service: Rc::new(service),
        })
    }
}

pub struct AuthMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for AuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody + 'static,
    S::Future: 'static,
    B::Error: std::error::Error,
{
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;
    type Response = ServiceResponse;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let uri = req.uri();
        let data = &req.app_data::<web::Data<AppData>>().unwrap();
        let need_token = uri != "/api/user/login" && data.config.comm.username.is_some();
        let token = req.headers().get("token").map(|v| v.to_str().unwrap_or("").to_owned());
        let service = self.service.clone();

        Box::pin(async move {
            if need_token {
                if let Some(token) = token {
                    if !token::is_valid(&token).await {
                        log::error!("authorize fail");
                        return Ok(req.error_response(error::ErrorUnauthorized("need authorized")));
                    }
                } else {
                    log::error!("need authorize");
                    return Ok(req.error_response(error::ErrorUnauthorized("need authorized")));
                }
            }
            let res = service.call(req).await?;
            Ok(res.map_body(|_, b| AnyBody::from_message(b)))
        })
    }
}
