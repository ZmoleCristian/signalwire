# SignalWire SDK for Rust

Unofficial SignalWire SDK for Rust. Async by default, optional blocking client behind a feature flag.

## Features

- JWT auth
- Phone number management (available, owned, buy)
- SMS send + delivery status
- Subproject (account) management
- Phone number lookup & validation (basic / carrier / CNAM)
- Built on `reqwest` with `rustls-tls` (no OpenSSL)
- Real `thiserror`-based error type — preserves status code, response body, transport errors
- 30s default timeout, `429 Too Many Requests` mapped to `RateLimited(Option<Duration>)`

## Install

```toml
[dependencies]
signalwire = "0.2"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Blocking client:

```toml
[dependencies]
signalwire = { version = "0.2", features = ["blocking"] }
```

## Configuration

```bash
# .env
SIGNALWIRE_SPACE_NAME=your_space
SIGNALWIRE_PROJECT_ID=your_project_id
SIGNALWIRE_API_KEY=your_api_key
```

## Async usage

### Init

```rust
use signalwire::{SignalWireClient, SignalWireError};

#[tokio::main]
async fn main() -> Result<(), SignalWireError> {
    let client = SignalWireClient::new("space", "project_id", "api_key")?;
    let jwt = client.get_jwt().await?;
    println!("jwt: {}", jwt.jwt_token);
    Ok(())
}
```

### Phone numbers

```rust
use signalwire::{PhoneNumberAvailableQueryParams, PhoneNumberOwnedFilterParams};

let q = PhoneNumberAvailableQueryParams::new().area_code("206").build();
let available = client.get_phone_numbers_available("US", &q).await?;

let owned = client
    .get_phone_numbers_owned(&PhoneNumberOwnedFilterParams::new().build())
    .await?;

let bought = client.buy_phone_number("+12065550100").await?;
```

### SMS

```rust
use signalwire::SmsMessage;

let msg = SmsMessage {
    from: "+12065550100".into(),
    to: "+12065550111".into(),
    body: "hello from rust".into(),
};

let resp = client.send_sms(&msg).await?;
println!("sid={} status={}", resp.sid, resp.get_status());

let status = client.get_message_status(&resp.sid).await?;
println!("status: {}", status.get_status()); // MessageStatus enum
```

### Subprojects

```rust
use signalwire::SubprojectQueryParams;

let list = client.list_subprojects(&SubprojectQueryParams::new().build()).await?;

let created = client.create_subproject("my-subproject").await?;
let updated = client.update_subproject(&created.sid, "renamed", None).await?;

// signalwire wants Status=closed before delete
client.update_subproject(&created.sid, "renamed", Some("closed")).await?;
client.delete_subproject(&created.sid).await?;

let nums = client
    .get_subproject_phone_numbers(
        &created.sid,
        &PhoneNumberOwnedFilterParams::new().build(),
    )
    .await?;
```

### Lookup

```rust
use signalwire::LookupKind;

let basic    = client.lookup_phone_number("+12065550100").await?;
let carrier  = client.lookup(&"+12065550100", LookupKind::Carrier).await?;
let cnam     = client.lookup(&"+12065550100", LookupKind::CallerName).await?;

// fields are Option<T> — no implicit defaults
if let Some(true) = basic.valid_number {
    println!("e164: {}", basic.e164.as_deref().unwrap_or("?"));
}
```

## Blocking usage

Same surface, sync. Built on `reqwest::blocking` — no hand-rolled tokio runtime per call.

```rust
use signalwire::{BlockingClient, SmsMessage};

fn main() -> Result<(), signalwire::SignalWireError> {
    let client = BlockingClient::new("space", "project_id", "api_key")?;

    let resp = client.send_sms(&SmsMessage {
        from: "+12065550100".into(),
        to: "+12065550111".into(),
        body: "sync hello".into(),
    })?;
    println!("sid: {}", resp.sid);

    let st = client.get_message_status(&resp.sid)?;
    println!("status: {}", st.get_status());

    Ok(())
}
```

## Error handling

```rust
pub enum SignalWireError {
    Http(reqwest::Error),                 // transport / TLS / timeout
    Unauthorized,                         // 401
    NotFound(String),                     // 404 + body
    RateLimited(Option<Duration>),        // 429 + Retry-After header
    Api { code: u16, body: String },      // 4xx/5xx
    Decode { source: serde_json::Error, body: String },
}
```

Match on it:

```rust
match client.send_sms(&msg).await {
    Ok(r) => println!("{}", r.sid),
    Err(SignalWireError::Unauthorized) => eprintln!("bad creds"),
    Err(SignalWireError::RateLimited(retry)) => eprintln!("backoff: {retry:?}"),
    Err(SignalWireError::Api { code, body }) => eprintln!("{code}: {body}"),
    Err(e) => eprintln!("{e}"),
}
```

## Custom HTTP client

Inject your own `reqwest::Client` (custom proxies, headers, TLS config):

```rust
let http = reqwest::Client::builder()
    .timeout(std::time::Duration::from_secs(60))
    .build()?;

let client = SignalWireClient::new("space", "id", "key")?
    .with_http_client(http);
```

## Tests

Live integration tests live in `tests/live.rs`, all `#[ignore]`-gated. Run manually:

```bash
cargo test --test live -- --ignored --nocapture jwt_async
```

`send_sms_costs_money` and `subproject_lifecycle` mutate real resources — keep them gated.

## License

[0BSD](LICENSE)

## Changelog

### 0.2.0
- Full rewrite. Edition 2024.
- One internal `handle<T>` request helper kills ~600 lines of boilerplate.
- Blocking client via `reqwest::blocking` (was: hand-rolled tokio runtime per call).
- `SignalWireError` preserves transport errors, status codes, response bodies. New `RateLimited(Option<Duration>)` variant for 429s.
- `SignalWireClient::new` / `BlockingClient::new` return `Result` — no `expect`/`unwrap` in the SDK.
- Private fields on the client (was: `pub api_key` etc.).
- `Daum` → `OwnedPhoneNumber`. Dead `#[serde(skip_deserializing)]` compat fields on `PhoneLookupResponse` removed.
- `LookupKind` enum + single `lookup(num, kind)` method (the three named methods kept as thin wrappers).
- One generic `QueryBuilder` replaces four duplicate query-param builders.
- Drops `dotenv`, `serde_derive`, `chrono`, `tokio` (runtime), `url` direct deps. `rustls-tls` instead of native-tls.
- 30s default request timeout.
- Tests moved to `tests/live.rs`, all `#[ignore]`-gated.

### 0.1.8
- Phone number lookup and validation, carrier + CNAM info.

### 0.1.7
- Subproject (account) management.

### 0.1.6
- SMS messaging + status checking + `MessageStatus` enum + `NotFound` error variant.

### 0.1.5
- Initial release: JWT auth, phone number management, blocking support.
