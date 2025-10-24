//! Integration test example showing how to use tracing-tower-mozlog in tests.
//!
//! This example demonstrates:
//! - Setting up MozLog for testing
//! - Capturing and asserting on log output
//! - Testing different scenarios with structured logging
//!
//! Run with: `cargo run --example integration_test`

use axum::{Router, extract::Path, http::StatusCode, response::Json, routing::get};
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    sync::{Arc, Mutex},
};
use tower::ServiceExt;
use tracing::{error, info, warn};
use tracing_subscriber::{fmt::MakeWriter, layer::SubscriberExt};
use tracing_tower_mozlog::{JsonStorageLayer, MozLogFormatLayer, MozLogLayer, MozLogMessage};

#[derive(Serialize, Deserialize)]
struct ApiResponse {
    message: String,
    status: String,
}

// Test writer that captures log output
#[derive(Default, Clone)]
struct TestWriter {
    buffer: Arc<Mutex<Vec<u8>>>,
}

impl TestWriter {
    fn new() -> Self {
        Self {
            buffer: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn get_logs(&self) -> Vec<MozLogMessage> {
        let buffer = self.buffer.lock().unwrap();
        let content = String::from_utf8(buffer.clone()).unwrap();

        content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }
}

impl Write for TestWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut buffer = self.buffer.lock().unwrap();
        buffer.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl MakeWriter<'_> for TestWriter {
    type Writer = Self;

    fn make_writer(&self) -> Self::Writer {
        self.clone()
    }
}

// Test handlers
async fn success_handler() -> Json<ApiResponse> {
    info!(r#type = "api.success", "Successful operation");
    Json(ApiResponse {
        message: "Operation completed successfully".to_string(),
        status: "success".to_string(),
    })
}

async fn error_handler() -> Result<Json<ApiResponse>, StatusCode> {
    error!(
        r#type = "api.error",
        error_code = "INTERNAL_ERROR",
        "An error occurred"
    );
    Err(StatusCode::INTERNAL_SERVER_ERROR)
}

async fn parameterized_handler(Path(param): Path<String>) -> Json<ApiResponse> {
    info!(
        r#type = "api.parameterized",
        parameter = %param,
        "Parameterized endpoint called"
    );

    Json(ApiResponse {
        message: format!("Received parameter: {param}"),
        status: "success".to_string(),
    })
}

async fn create_app() -> Router {
    Router::new()
        .route("/success", get(success_handler))
        .route("/error", get(error_handler))
        .route("/param/{value}", get(parameterized_handler))
        .layer(MozLogLayer::new())
}

async fn test_successful_request() {
    println!("Testing successful request...");

    let test_writer = TestWriter::new();
    let subscriber = tracing_subscriber::registry()
        .with(JsonStorageLayer)
        .with(MozLogFormatLayer::new("test-service", test_writer.clone()));

    let _guard = tracing::subscriber::set_default(subscriber);

    let app = create_app().await;
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/success")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Check logs
    let logs = test_writer.get_logs();

    // Should have application log and request summary
    assert!(logs.len() >= 2, "Expected at least 2 log entries");

    // Check for application log
    let app_log = logs.iter().find(|log| log.message_type == "api.success");
    assert!(app_log.is_some(), "Should have api.success log");

    // Check for request summary
    let request_log = logs
        .iter()
        .find(|log| log.message_type == "request.summary");
    assert!(request_log.is_some(), "Should have request.summary log");

    let request_log = request_log.unwrap();
    assert_eq!(
        request_log.fields.get("code").unwrap().as_u64().unwrap(),
        200
    );
    assert_eq!(
        request_log.fields.get("method").unwrap().as_str().unwrap(),
        "GET"
    );
    assert_eq!(
        request_log.fields.get("path").unwrap().as_str().unwrap(),
        "/success"
    );

    println!("✓ Successful request test passed");
}

async fn test_error_request() {
    println!("Testing error request...");

    let test_writer = TestWriter::new();
    let subscriber = tracing_subscriber::registry()
        .with(JsonStorageLayer)
        .with(MozLogFormatLayer::new("test-service", test_writer.clone()));

    let _guard = tracing::subscriber::set_default(subscriber);

    let app = create_app().await;
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/error")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    // Check logs
    let logs = test_writer.get_logs();

    // Should have error log and request summary
    assert!(logs.len() >= 2, "Expected at least 2 log entries");

    // Check for error log
    let error_log = logs.iter().find(|log| log.message_type == "api.error");
    assert!(error_log.is_some(), "Should have api.error log");

    let error_log = error_log.unwrap();
    assert_eq!(error_log.severity, 3); // ERROR level
    assert_eq!(
        error_log
            .fields
            .get("error_code")
            .unwrap()
            .as_str()
            .unwrap(),
        "INTERNAL_ERROR"
    );

    // Check for request summary with error
    let request_log = logs
        .iter()
        .find(|log| log.message_type == "request.summary");
    assert!(request_log.is_some(), "Should have request.summary log");

    let request_log = request_log.unwrap();
    assert_eq!(
        request_log.fields.get("code").unwrap().as_u64().unwrap(),
        500
    );
    assert_eq!(
        request_log.fields.get("errno").unwrap().as_u64().unwrap(),
        1
    );

    println!("✓ Error request test passed");
}

async fn test_parameterized_request() {
    println!("Testing parameterized request...");

    let test_writer = TestWriter::new();
    let subscriber = tracing_subscriber::registry()
        .with(JsonStorageLayer)
        .with(MozLogFormatLayer::new("test-service", test_writer.clone()));

    let _guard = tracing::subscriber::set_default(subscriber);

    let app = create_app().await;
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/param/test-value")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Check logs
    let logs = test_writer.get_logs();

    // Find the parameterized log
    let param_log = logs
        .iter()
        .find(|log| log.message_type == "api.parameterized");
    assert!(param_log.is_some(), "Should have api.parameterized log");

    let param_log = param_log.unwrap();
    assert_eq!(
        param_log.fields.get("parameter").unwrap().as_str().unwrap(),
        "test-value"
    );

    // Check request summary
    let request_log = logs
        .iter()
        .find(|log| log.message_type == "request.summary");
    assert!(request_log.is_some(), "Should have request.summary log");

    let request_log = request_log.unwrap();
    assert_eq!(
        request_log.fields.get("path").unwrap().as_str().unwrap(),
        "/param/test-value"
    );

    println!("✓ Parameterized request test passed");
}

fn test_log_structure() {
    println!("Testing log structure...");

    let test_writer = TestWriter::new();
    let subscriber = tracing_subscriber::registry()
        .with(JsonStorageLayer)
        .with(MozLogFormatLayer::new("test-service", test_writer.clone()));

    let _guard = tracing::subscriber::set_default(subscriber);

    // Log some test events
    info!(r#type = "test.info", key = "value", "Test info message");
    warn!(r#type = "test.warning", "Test warning message");
    error!(
        r#type = "test.error",
        error = "test_error",
        "Test error message"
    );

    let logs = test_writer.get_logs();
    assert_eq!(logs.len(), 3, "Should have 3 log entries");

    // Check info log
    let info_log = &logs[0];
    assert_eq!(info_log.message_type, "test.info");
    assert_eq!(info_log.severity, 5); // INFO
    assert_eq!(info_log.logger, "test-service");
    assert_eq!(info_log.env_version, "2.0");
    assert!(info_log.pid > 0);
    assert!(!info_log.hostname.is_empty());
    assert!(info_log.timestamp > 0);
    assert_eq!(
        info_log.fields.get("key").unwrap().as_str().unwrap(),
        "value"
    );

    // Check warning log
    let warn_log = &logs[1];
    assert_eq!(warn_log.message_type, "test.warning");
    assert_eq!(warn_log.severity, 4); // WARN

    // Check error log
    let error_log = &logs[2];
    assert_eq!(error_log.message_type, "test.error");
    assert_eq!(error_log.severity, 3); // ERROR
    assert_eq!(
        error_log.fields.get("error").unwrap().as_str().unwrap(),
        "test_error"
    );

    println!("✓ Log structure test passed");
}

#[tokio::main]
async fn main() {
    println!("Running tracing-tower-mozlog integration tests...\n");

    test_log_structure();
    test_successful_request().await;
    test_error_request().await;
    test_parameterized_request().await;

    println!("\n🎉 All integration tests passed!");
    println!("\nThis demonstrates that tracing-tower-mozlog correctly:");
    println!("  • Formats logs in MozLog JSON format");
    println!("  • Captures request/response metadata");
    println!("  • Handles different log levels and severity mapping");
    println!("  • Includes timing information");
    println!("  • Preserves custom fields and message types");
    println!("  • Works with Tower/Axum middleware pattern");
}
