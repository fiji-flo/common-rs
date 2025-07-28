use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use pretty_assertions::assert_eq;
use serde_json::json;
use tower::ServiceExt;

use crate::utils::{log_test_async, LogWatcher};
use tracing_tower_mozlog::MozLogLayer;

async fn handler_status_echo(
    axum::extract::Path(status): axum::extract::Path<u16>,
) -> Result<Response, StatusCode> {
    match StatusCode::from_u16(status) {
        Ok(status_code) => Ok((status_code, "").into_response()),
        Err(_) => Err(StatusCode::BAD_REQUEST),
    }
}

#[derive(Debug)]
struct TestError;

impl IntoResponse for TestError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, "test error").into_response()
    }
}

async fn handler_error() -> Result<Response, TestError> {
    Err(TestError)
}

#[tokio::test]
async fn test_it_logs_requests() {
    let mut log_watcher: LogWatcher = log_test_async(|| async {
        let app = Router::new()
            .route("/{status}", get(handler_status_echo))
            .layer(MozLogLayer::new());

        // Test successful request
        let request = Request::builder()
            .method(Method::GET)
            .uri("/200")
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Test client error
        let request = Request::builder()
            .method(Method::GET)
            .uri("/400")
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // Test server error
        let request = Request::builder()
            .method(Method::GET)
            .uri("/500")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    })
    .await;

    assert!(
        log_watcher.has(|event| {
            event.severity == 5
                && event.message_type == "request.summary"
                && event.fields.get("code") == Some(&json!(200))
        }),
        "should log successful responses"
    );
    assert!(
        log_watcher.has(|event| {
            event.severity == 5
                && event.fields.get("code") == Some(&json!(400))
                && event.message_type == "request.summary"
        }),
        "should log client errors"
    );
    assert!(
        log_watcher.has(|event| {
            event.severity == 5
                && event.fields.get("code") == Some(&json!(500))
                && event.message_type == "request.summary"
        }),
        "should log server errors"
    );
}

#[tokio::test]
async fn test_request_summary_has_recommended_fields() {
    let mut log_watcher: LogWatcher = log_test_async(|| async {
        let app = Router::new()
            .route("/{status}", get(handler_status_echo))
            .layer(MozLogLayer::new());

        let request = Request::builder()
            .method(Method::GET)
            .uri("/200")
            .header("User-Agent", "A Test Client")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    })
    .await;

    let event = log_watcher
        .events()
        .iter()
        .find(|event| event.message_type == "request.summary")
        .expect("Could not find request.summary event");

    // Check that required fields are present
    assert_eq!(event.message_type, "request.summary");
    assert_eq!(event.logger, "test-logger");
    assert_eq!(event.env_version, "2.0");
    assert_eq!(event.severity, 5);

    // Check specific fields
    assert_eq!(*event.fields.get("agent").unwrap(), json!("A Test Client"));
    assert_eq!(*event.fields.get("path").unwrap(), json!("/200"));
    assert_eq!(*event.fields.get("method").unwrap(), json!("GET"));
    assert_eq!(*event.fields.get("code").unwrap(), json!(200));
    assert_eq!(*event.fields.get("spans").unwrap(), json!("request"));

    // Check that timing fields are present
    assert!(event.fields.contains_key("rid"));
    assert!(event.fields.contains_key("t"));
    assert!(event.fields.contains_key("t_ns"));

    // Verify that the request ID is a valid UUID format
    let rid = event.fields.get("rid").unwrap().as_str().unwrap();
    assert!(rid.len() == 36 && rid.chars().filter(|&c| c == '-').count() == 4);
}

#[tokio::test]
async fn test_it_logs_controlled_errors() {
    let mut log_watcher: LogWatcher = log_test_async(|| async {
        let app = Router::new()
            .route("/", get(handler_error))
            .layer(MozLogLayer::new());

        let request = Request::builder()
            .method(Method::GET)
            .uri("/")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    })
    .await;

    assert!(
        log_watcher.has(|event| {
            event.severity == 5
                && event.message_type == "request.summary"
                && event.fields.get("code") == Some(&json!(500))
                && event.fields.get("errno") == Some(&json!(1))
        }),
        "errors are still logged with INFO level request.summary and errno=1"
    );
}

#[tokio::test]
async fn test_request_summary_does_not_include_query_strings() {
    let mut log_watcher: LogWatcher = log_test_async(|| async {
        let app = Router::new()
            .route("/{status}", get(handler_status_echo))
            .layer(MozLogLayer::new());

        let request = Request::builder()
            .method(Method::GET)
            .uri("/200?a=1&b=2")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    })
    .await;

    let event = log_watcher
        .events()
        .iter()
        .find(|event| event.message_type == "request.summary")
        .expect("Could not find request.summary event");

    assert_eq!(
        *event.fields.get("path").unwrap(),
        json!("/200"),
        "should not include query string in logged path"
    );
}

#[tokio::test]
async fn test_different_http_methods() {
    let mut log_watcher: LogWatcher = log_test_async(|| async {
        async fn post_handler() -> &'static str {
            "POST response"
        }

        async fn put_handler() -> &'static str {
            "PUT response"
        }

        let app = Router::new()
            .route("/post", axum::routing::post(post_handler))
            .route("/put", axum::routing::put(put_handler))
            .layer(MozLogLayer::new());

        // Test POST request
        let request = Request::builder()
            .method(Method::POST)
            .uri("/post")
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Test PUT request
        let request = Request::builder()
            .method(Method::PUT)
            .uri("/put")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    })
    .await;

    // Check POST request was logged
    assert!(
        log_watcher.has(|event| {
            event.message_type == "request.summary"
                && event.fields.get("method") == Some(&json!("POST"))
                && event.fields.get("path") == Some(&json!("/post"))
        }),
        "should log POST requests correctly"
    );

    // Check PUT request was logged
    assert!(
        log_watcher.has(|event| {
            event.message_type == "request.summary"
                && event.fields.get("method") == Some(&json!("PUT"))
                && event.fields.get("path") == Some(&json!("/put"))
        }),
        "should log PUT requests correctly"
    );
}
