use actix_web::{
    Error, HttpResponse,
    body::EitherBody,
    dev::{self, Service, ServiceRequest, ServiceResponse, Transform},
    http::StatusCode,
};
use futures_util::future::LocalBoxFuture;
use std::future::{Ready, ready};

pub struct RedirectUnauthorized;

impl<S, B> Transform<S, ServiceRequest> for RedirectUnauthorized
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = RedirectUnauthorizedMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RedirectUnauthorizedMiddleware { service }))
    }
}

pub struct RedirectUnauthorizedMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for RedirectUnauthorizedMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    dev::forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let fut = self.service.call(req);

        Box::pin(async move {
            let res = fut.await?;

            if res.status() == StatusCode::UNAUTHORIZED {
                let (req_parts, _) = res.into_parts();
                let redirect_response = HttpResponse::SeeOther()
                    .insert_header((actix_web::http::header::LOCATION, "/auth/signin"))
                    .finish()
                    .map_into_right_body();

                return Ok(ServiceResponse::new(req_parts, redirect_response));
            }

            Ok(res.map_into_left_body())
        })
    }
}
