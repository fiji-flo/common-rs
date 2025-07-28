# tracing-tower-mozlog

[![License: MPL 2.0]][mpl 2.0]
[![Build Status]][circleci]
[![version-badge::tracing-tower-mozlog]][crates.io::tracing-tower-mozlog]
[![rustdoc-badge::tracing-tower-mozlog]][docs::tracing-tower-mozlog]

[license: mpl 2.0]: https://img.shields.io/badge/License-MPL%202.0-blue.svg
[mpl 2.0]: https://opensource.org/licenses/MPL-2.0
[build status]: https://img.shields.io/circleci/build/github/mozilla-services/common-rs
[circleci]: https://app.circleci.com/pipelines/github/mozilla-services/common-rs

[version-badge::tracing-tower-mozlog]: https://img.shields.io/crates/v/tracing-tower-mozlog.svg
[crates.io::tracing-tower-mozlog]: https://crates.io/crates/tracing-tower-mozlog
[docs::tracing-tower-mozlog]: https://docs.rs/tracing-tower-mozlog
[rustdoc-badge::tracing-tower-mozlog]: https://img.shields.io/docsrs/tracing-tower-mozlog

Support for [tracing][] in [Tower][]/[Axum][] apps that target Mozilla's [MozLog][].

[tracing]: https://tracing.rs/tracing/
[Tower]: https://github.com/tower-rs/tower
[Axum]: https://github.com/tokio-rs/axum
[MozLog]: https://wiki.mozilla.org/Firefox/Services/Logging

## Features

- **MozLog Format**: Structured JSON logging that conforms to Mozilla's logging standard
- **Tower Integration**: Works with any Tower-based framework (Axum, Warp, etc.)
- **Request Tracing**: Automatic request/response logging with timing and metadata
- **Span Support**: Hierarchical span information included in log events
- **Configurable**: Flexible configuration for different service needs

## Usage

### Basic Setup

Add this to your `Cargo.toml`:

```toml
[dependencies]
tracing-tower-mozlog = "0.1"
tracing-subscriber = "0.3"
axum = "0.8"
tokio = { version = "1", features = ["full"] }
```

### Subscriber Configuration

Set up the MozLog subscriber:

```rust
use tracing_tower_mozlog::{JsonStorageLayer, MozLogFormatLayer};
use tracing_subscriber::layer::SubscriberExt;

fn main() {
    let subscriber = tracing_subscriber::registry()
        .with(JsonStorageLayer)
        .with(MozLogFormatLayer::new("my-service", std::io::stdout()));

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set subscriber");

    // Your application code here
}
```

### Axum Integration

Use with Axum applications:

```rust
use axum::{body::Body, Router, routing::get};
use tower::ServiceBuilder;
use tracing_tower_mozlog::MozLogLayer;

#[tokio::main]
async fn main() {
    // Set up subscriber (as shown above)

    let app: Router<(), Body> = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/health", get(|| async { "OK" }))
        .layer(ServiceBuilder::new().layer(MozLogLayer::new()));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();

    axum::serve(listener, app).await.unwrap();
}
```

### Generic Tower Service

Use with any Tower service:

```rust
use tower::{ServiceBuilder, service_fn};
use tracing_tower_mozlog::MozLogLayer;
use http::Response;

let service = ServiceBuilder::new()
    .layer(MozLogLayer::new())
    .service(service_fn(|req| async {
        // Your service logic
        Ok::<_, std::convert::Infallible>(Response::new("Hello"))
    }));
```

## Message Types

MozLog requires message types to categorize log events. Use the `r#type` field:

```rust
use tracing::info;

// Application events
info!(r#type = "app.startup", "Service started successfully");

// Authentication events
info!(r#type = "auth.login.success", user_id = "12345", "User logged in");

// Error events
error!(r#type = "auth.login.failure", error = "invalid_password", "Login failed");
```

## Request Logging

The middleware automatically logs request summaries with these fields:

- `method`: HTTP method (GET, POST, etc.)
- `path`: Request path (without query string)
- `code`: HTTP status code
- `rid`: Unique request ID
- `agent`: User-Agent header
- `t`: Request duration in milliseconds
- `t_ns`: Request duration in nanoseconds
- `errno`: Error number (1 for errors, empty for success)
- `msg`: Error message (if applicable)

Example log output:

```json
{
  "Timestamp": 1640995200000000000,
  "Type": "request.summary",
  "Logger": "my-service",
  "Hostname": "web-server-01",
  "EnvVersion": "2.0",
  "Severity": 5,
  "Pid": 1234,
  "Fields": {
    "method": "GET",
    "path": "/api/users",
    "code": 200,
    "rid": "550e8400-e29b-41d4-a716-446655440000",
    "agent": "Mozilla/5.0...",
    "t": 45,
    "t_ns": 45123456,
    "spans": "request"
  }
}
```

## Span Integration

The library automatically includes span information in log events:

```rust
use tracing::{info, info_span};

let span = info_span!("user_operation", user_id = "12345");
let _guard = span.enter();

info!(r#type = "user.profile.updated", "Profile updated successfully");
// This event will include user_id from the span
```

## Compatibility

- **Tower**: 0.4+
- **Axum**: 0.7+
- **Tokio**: 1.0+
- **Tracing**: 0.1+

## License

This project is licensed under the Mozilla Public License 2.0 - see the [LICENSE](../LICENSE) file for details.

## Examples

Check out the [examples directory](examples/) for complete working examples:

- [`axum_basic.rs`](examples/axum_basic.rs) - Basic Axum integration with different endpoints
- [`integration_test.rs`](examples/integration_test.rs) - Testing setup and log assertion patterns

Run examples with:
```
cargo run --example axum_basic
cargo run --example integration_test
```
