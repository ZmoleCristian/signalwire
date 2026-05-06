//! Error type for the SignalWire SDK.

use std::time::Duration;

use thiserror::Error;

/// Every public method on the SDK returns `Result<_, SignalWireError>`.
///
/// Variants preserve enough context to handle each failure mode without
/// stringly-typed parsing of the response body.
#[derive(Debug, Error)]
pub enum SignalWireError {
    /// Transport-level failure (DNS, TLS handshake, timeout, connection reset).
    #[error("http transport error: {0}")]
    Http(#[from] reqwest::Error),

    /// `401 Unauthorized` — bad project id / api key.
    #[error("unauthorized — bad project id or api key")]
    Unauthorized,

    /// `404 Not Found`. Body included for context.
    #[error("not found: {0}")]
    NotFound(String),

    /// `429 Too Many Requests`. Carries the parsed `Retry-After` header if
    /// present and parseable as seconds. `None` means the server didn't tell
    /// us how long to back off.
    #[error("rate limited (retry_after={:?})", .0)]
    RateLimited(Option<Duration>),

    /// Any other 4xx/5xx response. Both the status code and raw body are kept.
    #[error("api error ({code}): {body}")]
    Api { code: u16, body: String },

    /// Response body did not parse as the expected JSON shape.
    /// `body` is the raw text returned by the server.
    #[error("decode error: {source}; body={body}")]
    Decode {
        #[source]
        source: serde_json::Error,
        body: String,
    },
}
