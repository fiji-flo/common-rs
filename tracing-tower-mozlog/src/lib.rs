//! # tracing-tower-mozlog
//!
//! Support for [tracing] in [Tower]/[Axum] apps that target [MozLog][].
//!
//! [tracing]: https://tracing.rs/tracing/
//! [Tower]: https://github.com/tower-rs/tower
//! [Axum]: https://github.com/tokio-rs/axum
//! [MozLog]: https://wiki.mozilla.org/Firefox/Services/Logging
//!
//! This crate provides a Tracing subscriber as well as a Tower middleware layer.
//! Both can be used independently, but using them together implements the full
//! recommended format for MozLog.
//!
//! ## Subscriber
//!
//! To use the subscriber, register it and a [`JsonStorageLayer`] with a
//! [`tracing_subscriber::Registry`]:
//!
//! ```
//! use tracing_tower_mozlog::{JsonStorageLayer, MozLogFormatLayer};
//! use tracing_subscriber::layer::SubscriberExt;
//!
//! let subscriber = tracing_subscriber::registry()
//!     .with(JsonStorageLayer)
//!     .with(MozLogFormatLayer::new("service-name", std::io::stdout));
//! ```
//!
//! This subscriber can then be registered with tracing using
//! [`tracing::subscriber::set_global_default`], or any other registration
//! method. It will manage formatting any events logged in MozLog JSON format.
//!
//! Fields defined on the enclosing spans of an event will be included when
//! logging an event. The event overrides the spans, and inner spans override
//! outer spans.
//!
//! ## Middleware
//!
//! To use the Tower middleware layer with Axum:
//!
//! ```rust,no_run
//! use tracing_tower_mozlog::MozLogLayer;
//! use axum::{body::Body, Router, routing::get};
//! use tower::ServiceBuilder;
//!
//! let app: Router = Router::new()
//!     .route("/", get(|| async { "Hello, World!" }))
//!     .layer(ServiceBuilder::new().layer(MozLogLayer::new()));
//! ```
//!
//! This middleware will emit `request.summary` events for each request as it is
//! completed, including timing information.
//!
//! ## Message Types
//!
//! MozLog expects all messages to have a type that defines the schema of their
//! fields. This can be specified with the `type` field while logging events.
//! Since `type` is a Rust reserved keyword, this can also be specified using a
//! raw-string-inspired format: `r#type = value`.
//!
//! ```rust
//! let format_error = "...";
//! tracing::warn!(
//!     r#type = "auth.login.invalid-email",
//!     %format_error,
//!     "A user attempted to register using an email in an invalid format"
//! );
//! ```
//!
//! Messages that don't include a `type` field will be assigned a type of
//! `<unknown>`. If a message contains both a `type` field and a `r#type` field,
//! the `type` field will take precedence.
//!
//! Notably, use of the standard `log` facade's macros will have an unknown type,
//! as well as most other logging that originates from libraries.
//!
//! ## MozLog extensions
//!
//! In addition to all standard MozLog fields, this crate always adds a `spans`
//! field to messages. This contains a comma-separated list of the names of the
//! spans enclosing the event, with the outermost span coming first. Top-level
//! events will have an empty string for this value.

#![warn(rustdoc::missing_crate_level_docs)]
#![warn(missing_docs)]

mod middleware;
mod subscriber;

pub use crate::middleware::{MozLogLayer, MozLogService};
pub use crate::subscriber::{MozLogFormatLayer, MozLogMessage};

/// A layer to collect information about Tracing spans and provide it to other layers.
///
/// This is a re-exported entry from [`tracing_bunyan_formatter`].
pub use tracing_bunyan_formatter::JsonStorageLayer;
