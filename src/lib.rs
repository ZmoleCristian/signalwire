//! Unofficial SignalWire SDK for Rust.
//!
//! Async by default. Enable the `blocking` feature for a sync client.
//!
//! # Quick start
//!
//! ```no_run
//! use signalwire::{SignalWireClient, SmsMessage};
//!
//! # async fn run() -> Result<(), signalwire::SignalWireError> {
//! let client = SignalWireClient::new("space", "project_id", "api_key")?;
//!
//! let msg = SmsMessage {
//!     from: "+12065550100".into(),
//!     to:   "+12065550111".into(),
//!     body: "hello".into(),
//! };
//! let resp = client.send_sms(&msg).await?;
//! println!("sid={} status={}", resp.sid, resp.get_status());
//! # Ok(()) }
//! ```
//!
//! # Modules
//!
//! - [`client`] — async [`SignalWireClient`].
//! - [`blocking`] (feature `blocking`) — sync [`BlockingClient`].
//! - [`errors`] — [`SignalWireError`].
//! - [`types`] — request/response types and query-param builders.

#![allow(clippy::new_without_default)]

pub mod client;
pub mod errors;
pub mod types;

#[cfg(feature = "blocking")]
pub mod blocking;

pub use client::SignalWireClient;
pub use errors::SignalWireError;
pub use types::*;

#[cfg(feature = "blocking")]
pub use blocking::BlockingClient;
