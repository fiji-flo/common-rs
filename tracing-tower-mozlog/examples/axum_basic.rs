//! Basic example of using tracing-tower-mozlog with Axum.
//!
//! This example demonstrates:
//! - Setting up the MozLog subscriber
//! - Using the Tower middleware with Axum
//! - Different types of log events
//!
//! Run with: `cargo run --example axum_basic`

use axum::{
    Router,
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

use tower::ServiceBuilder;
use tracing::{error, info, warn};
use tracing_subscriber::layer::SubscriberExt;
use tracing_tower_mozlog::{JsonStorageLayer, MozLogFormatLayer, MozLogLayer};

#[derive(Serialize, Deserialize)]
struct User {
    id: u32,
    name: String,
    email: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    code: u16,
}

// Handler that always succeeds
async fn health_check() -> &'static str {
    info!(r#type = "health.check", "Health check endpoint called");
    "OK"
}

// Handler that demonstrates different response codes
async fn get_user(Path(user_id): Path<u32>) -> Result<Json<User>, StatusCode> {
    info!(
        r#type = "user.fetch.attempt",
        user_id = user_id,
        "Attempting to fetch user"
    );

    match user_id {
        1 => {
            let user = User {
                id: 1,
                name: "Alice".to_string(),
                email: "alice@example.com".to_string(),
            };
            info!(
                r#type = "user.fetch.success",
                user_id = user_id,
                "User fetched successfully"
            );
            Ok(Json(user))
        }
        404 => {
            warn!(
                r#type = "user.fetch.not_found",
                user_id = user_id,
                "User not found"
            );
            Err(StatusCode::NOT_FOUND)
        }
        500 => {
            error!(
                r#type = "user.fetch.error",
                user_id = user_id,
                error = "database_connection_failed",
                "Internal server error while fetching user"
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
        _ => {
            warn!(
                r#type = "user.fetch.invalid_id",
                user_id = user_id,
                "Invalid user ID provided"
            );
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

// Handler that creates a user
async fn create_user(Json(payload): Json<User>) -> Result<Json<User>, impl IntoResponse> {
    info!(
        r#type = "user.create.attempt",
        user_id = payload.id,
        email = %payload.email,
        "Attempting to create user"
    );

    // Simulate validation
    if payload.email.is_empty() {
        warn!(
            r#type = "user.create.validation_failed",
            user_id = payload.id,
            field = "email",
            "Email validation failed"
        );
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Email is required".to_string(),
                code: 400,
            }),
        ));
    }

    info!(
        r#type = "user.create.success",
        user_id = payload.id,
        email = %payload.email,
        "User created successfully"
    );

    Ok(Json(payload))
}

#[tokio::main]
async fn main() {
    // Set up the MozLog subscriber
    let subscriber = tracing_subscriber::registry()
        .with(JsonStorageLayer)
        .with(MozLogFormatLayer::new("axum-example", std::io::stdout));

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set subscriber");

    info!(
        r#type = "app.startup",
        version = env!("CARGO_PKG_VERSION"),
        "Starting Axum server with MozLog"
    );

    // Build the application with routes and middleware
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/users/{id}", get(get_user))
        .route("/users", post(create_user))
        .layer(ServiceBuilder::new().layer(MozLogLayer::new()));

    info!(
        r#type = "app.ready",
        address = "127.0.0.1:3000",
        "Server is ready to accept connections"
    );

    println!("Server running on http://127.0.0.1:3000");
    println!("Try these endpoints:");
    println!("  GET  /health        - Health check");
    println!("  GET  /users/1       - Get user (success)");
    println!("  GET  /users/404     - Get user (not found)");
    println!("  GET  /users/500     - Get user (server error)");
    println!("  GET  /users/999     - Get user (bad request)");
    println!("  POST /users         - Create user");
    println!();
    println!("Example POST request:");
    println!(
        r#"  curl -X POST http://127.0.0.1:3000/users -H "Content-Type: application/json" -d '{{"id": 2, "name": "Bob", "email": "bob@example.com"}}'"#
    );

    // Run the server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("Failed to bind to address");

    axum::serve(listener, app)
        .await
        .expect("Server failed to start");
}
