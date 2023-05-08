//! Make an http span
// Straight up copied from "I won free Load Testing"
// TODO: deduplicate with tube

use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::sync::futures;

use http::{Request, Response};
use hyper::Body;
use tower::{Layer, Service};
use tower_request_id::RequestId;
use tracing::{field, info_span, instrument::Instrumented, Instrument, Span};

/// Layer for [IncomingHttpSpanService]
#[derive(Default)]
pub struct IncomingHttpSpanLayer {}

impl<S> Layer<S> for IncomingHttpSpanLayer
where
    S: Service<Request<Body>> + Clone + Send + 'static,
{
    type Service = IncomingHttpSpanService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        IncomingHttpSpanService { inner }
    }
}

/// Extracts opentelemetry context from HTTP headers
#[derive(Clone)]
pub struct IncomingHttpSpanService<S>
where
    S: Service<Request<Body>> + Clone + Send + 'static,
{
    inner: S,
}

impl<S, B> Service<Request<Body>> for IncomingHttpSpanService<S>
where
    S: Service<Request<Body>, Response = Response<B>> + Clone + Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = PostFuture<Instrumented<S::Future>, B, S::Error>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let user_agent = req
            .headers()
            .get("user-agent")
            .and_then(|s| s.to_str().ok())
            .unwrap_or("");

        let host = req
            .headers()
            .get("host")
            .and_then(|s| s.to_str().ok())
            .unwrap_or("");

        let sec_ch_ua_mobile = req
            .headers()
            .get("sec-ch-ua-mobile")
            .and_then(|s| s.to_str().ok())
            .unwrap_or("");

        let sec_ch_ua_platform = req
            .headers()
            .get("sec-ch-ua-platform")
            .and_then(|s| s.to_str().ok())
            .unwrap_or("");

        let span = info_span!(
            "http request",
            otel.name = %req.uri().path(),
            otel.kind = "server",
            http.method = %req.method(),
            http.url = %req.uri(),
            http.status_code = field::Empty,
            http.user_agent = &user_agent,
            http.host = &host,
            http.sec_ch_ua_mobile = &sec_ch_ua_mobile,
            http.sec_ch_ua_platform = &sec_ch_ua_platform,
            request_id = field::Empty,
            user_id = field::Empty,
        );

        if let Some(id) = req.extensions().get::<RequestId>() {
            span.record("request_id", &id.to_string().as_str());
        }

        let fut = {
            let _guard = span.enter();
            self.inner.call(req)
        };
        PostFuture {
            inner: fut.instrument(span.clone()),
            span,
        }
    }
}

pin_project_lite::pin_project! {
    /// Future that records http status code
    pub struct PostFuture<F, B, E>
    where
        F: Future<Output = Result<Response<B>, E>>,
    {
        #[pin]
        inner: F,
        span: Span,
    }
}

impl<F, B, E> Future for PostFuture<F, B, E>
where
    F: Future<Output = Result<Response<B>, E>>,
{
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let res = futures::ready!(this.inner.poll(cx));
        if let Ok(res) = &res {
            this.span.record("http.status_code", &res.status().as_u16());
        }
        res.into()
    }
}
