use crate::{AuthnBackend, session::Session};
use axum::http::{self, Request, Response};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

pub trait Service<Request> {
    type Response;
    type Error;
    type Future: Future<Output = Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>>;

    fn call(&mut self, req: Request) -> Self::Future;
}

impl<'a, S, Request> Service<Request> for &'a mut S
where
    S: Service<Request> + 'a,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        (**self).poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        (**self).call(req)
    }
}

impl<S, Request> Service<Request> for Box<S>
where
    S: Service<Request> + ?Sized,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), S::Error>> {
        (**self).poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> S::Future {
        (**self).call(request)
    }
}

#[derive(Debug, Clone)]
pub struct AuthManager<S, Backend: AuthnBackend> {
    inner: S,
    backend: Backend,
    data_key: &'static str,
}

impl<S, Backend: AuthnBackend> AuthManager<S, Backend> {
    pub fn new(inner: S, backend: Backend, data_key: &'static str) -> Self {
        Self {
            inner,
            backend,
            data_key,
        }
    }
}

impl<ReqBody, ResBody, S, Backend> Service<Request<ReqBody>> for AuthManager<S, Backend>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    ReqBody: Send + 'static,
    ResBody: Default + Send,
    Backend: AuthnBackend + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let span = tracing::info_span!("call", user.id = tracing::field::Empty);

        let backend = self.backend.clone();
        let data_key = self.data_key;

        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        Box::pin(async move {
            let Some(session) = req.extensions().get::<Session>().cloned() else {
                tracing::error!("session not fount in request extensions");
                let mut res = Response::default();
                *res.status_mut() = http::StatusCode::INTERNAL_SERVER_ERROR;
                return Ok(res);
            };

            todo!();
        })
    }
}
