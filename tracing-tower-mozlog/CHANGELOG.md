# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2024-01-15

### Added
- Initial release of `tracing-tower-mozlog`
- Tower middleware layer for request/response logging
- MozLog format subscriber for structured JSON logging
- Support for Tower-based frameworks (Axum, Warp, etc.)
- Automatic request timing and metadata collection
- Span integration for hierarchical logging context
- UUID-based request ID generation
- User-Agent header capture
- HTTP status code and error logging
- Configurable service name and output destination

### Features
- `MozLogLayer` - Tower middleware for request logging
- `MozLogService` - Tower service wrapper
- `MozLogFormatLayer` - Tracing subscriber for MozLog format
- `MozLogMessage` - Structured log message format
- Re-exported `JsonStorageLayer` from `tracing-bunyan-formatter`

[Unreleased]: https://github.com/mozilla-services/common-rs/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/mozilla-services/common-rs/releases/tag/v0.1.0