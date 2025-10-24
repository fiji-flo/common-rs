//! Tower middleware for request/response cycle logging.

use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Instant,
};

use http::{Request, Response};
use pin_project_lite::pin_project;
use tower_layer::Layer;
use tower_service::Service;
use tracing::Span;
use uuid::Uuid;

/// Tower layer that implements the request/response cycle logging
/// required by MozLog.
///
/// This layer will emit `request.summary` events for each request as it is
/// completed, including timing information.
///
/// # Example
///
/// ```rust,no_run
/// use tracing_tower_mozlog::MozLogLayer;
/// use axum::{body::Body, Router, routing::get};
/// use tower::ServiceBuilder;
///
/// let app: Router = Router::new()
///     .route("/", get(|| async { "Hello, World!" }))
///     .layer(ServiceBuilder::new().layer(MozLogLayer::new()));
/// ```
#[derive(Clone, Default)]
pub struct MozLogLayer;

impl MozLogLayer {
    /// Create a new MozLog layer.
    pub fn new() -> Self {
        Self
    }
}

impl<S> Layer<S> for MozLogLayer {
    type Service = MozLogService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        MozLogService { inner }
    }
}

/// Tower service that wraps another service to provide MozLog request logging.
#[derive(Clone)]
pub struct MozLogService<S> {
    inner: S,
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for MozLogService<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + Clone,
    S::Error: std::fmt::Display,
    ReqBody: Send + 'static,
    ResBody: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = MozLogFuture<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let start_time = Instant::now();
        let request_id = generate_request_id();

        let method = req.method().as_str();
        let path = req.uri().path();
        let user_agent = req
            .headers()
            .get("user-agent")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");

        let span = tracing::info_span!(
            "request",
            method = %method,
            path = %path,
            code = tracing::field::Empty,
            rid = %request_id,
            errno = tracing::field::Empty,
            agent = %user_agent,
            msg = tracing::field::Empty,
            lang = tracing::field::Empty,
            uid = tracing::field::Empty,
            t = tracing::field::Empty,
            t_ns = tracing::field::Empty,
        );

        let future = self.inner.call(req);

        MozLogFuture {
            inner: future,
            span,
            start_time,
        }
    }
}

pin_project! {
    /// Future returned by [`MozLogService`].
    pub struct MozLogFuture<F> {
        #[pin]
        inner: F,
        span: Span,
        start_time: Instant,
    }
}

impl<F, ResBody, E> Future for MozLogFuture<F>
where
    F: Future<Output = Result<Response<ResBody>, E>>,
    E: std::fmt::Display,
{
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let span = this.span;
        let start_time = *this.start_time;

        // Instrument the inner future with the span
        let _guard = span.enter();

        match this.inner.poll(cx) {
            Poll::Ready(result) => {
                let elapsed = start_time.elapsed();
                span.record("t", elapsed.as_millis() as u32);
                span.record("t_ns", elapsed.as_nanos() as u64);

                match &result {
                    Ok(response) => {
                        let status = response.status();
                        span.record("code", status.as_u16());

                        if status.is_client_error() || status.is_server_error() {
                            span.record("errno", 1);
                            span.record("msg", format!("HTTP {}", status.as_u16()));
                        }
                    }
                    Err(error) => {
                        span.record("errno", 1);
                        span.record("msg", tracing::field::display(error));
                        span.record("code", 500u16);
                    }
                }

                // Log the request summary within the span context
                tracing::info!(r#type = "request.summary");
                Poll::Ready(result)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Generate a unique request ID.
fn generate_request_id() -> Uuid {
    Uuid::new_v4()
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::{Method, StatusCode};
    use std::convert::Infallible;
    use tower::{ServiceExt, service_fn};

    #[tokio::test]
    async fn test_successful_request() {
        let service =
            service_fn(|_req: Request<()>| async { Ok::<_, Infallible>(Response::new(())) });

        let service = MozLogLayer::new().layer(service);

        let request = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .body(())
            .unwrap();

        let response = service.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_error_request() {
        let service =
            service_fn(|_req: Request<()>| async { Err::<Response<()>, _>("test error") });

        let service = MozLogLayer::new().layer(service);

        let request = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .body(())
            .unwrap();

        let result = service.oneshot(request).await;
        assert!(result.is_err());
    }
}
