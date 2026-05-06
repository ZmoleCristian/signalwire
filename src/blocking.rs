//! Blocking [`BlockingClient`] — only available with the `blocking` feature.
//!
//! Mirrors [`SignalWireClient`](crate::SignalWireClient) one-for-one. Backed
//! by `reqwest::blocking`, so calls run on the current OS thread without any
//! tokio runtime.

use std::time::Duration;

use reqwest::StatusCode;
use reqwest::blocking::{Client as HttpClient, Response};
use serde::de::DeserializeOwned;

use crate::{errors::SignalWireError, types::*};

/// Default per-request timeout applied to the bundled HTTP client.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Synchronous client for the SignalWire REST API.
///
/// Cheap to clone — the inner `reqwest::blocking::Client` shares connection
/// state across clones.
#[derive(Debug, Clone)]
pub struct BlockingClient {
    space_name: String,
    project_id: String,
    api_key: String,
    http: HttpClient,
}

impl BlockingClient {
    /// Build a blocking client with the default `reqwest::blocking::Client`
    /// (30s timeout, rustls-tls).
    pub fn new(space_name: impl Into<String>, project_id: impl Into<String>, api_key: impl Into<String>) -> Result<Self, SignalWireError> {
        let http = HttpClient::builder().timeout(DEFAULT_TIMEOUT).build()?;
        Ok(Self {
            space_name: space_name.into(),
            project_id: project_id.into(),
            api_key: api_key.into(),
            http,
        })
    }

    /// Swap in a custom `reqwest::blocking::Client`.
    pub fn with_http_client(mut self, http: HttpClient) -> Self {
        self.http = http;
        self
    }

    /// Project ID this client authenticates as.
    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    /// SignalWire space (subdomain) this client targets.
    pub fn space_name(&self) -> &str {
        &self.space_name
    }

    fn url(&self, path: &str) -> String {
        format!("https://{}.signalwire.com{}", self.space_name, path)
    }

    fn auth(&self, b: reqwest::blocking::RequestBuilder) -> reqwest::blocking::RequestBuilder {
        b.basic_auth(&self.project_id, Some(&self.api_key))
    }

    /// Translate a JSON response into `T` or the right `SignalWireError` variant.
    fn handle<T: DeserializeOwned>(resp: Response) -> Result<T, SignalWireError> {
        let status = resp.status();

        if status == StatusCode::TOO_MANY_REQUESTS {
            let retry = resp
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .map(Duration::from_secs);
            return Err(SignalWireError::RateLimited(retry));
        }

        let body = resp.text()?;

        match status {
            StatusCode::UNAUTHORIZED => Err(SignalWireError::Unauthorized),
            StatusCode::NOT_FOUND => Err(SignalWireError::NotFound(body)),
            s if s.is_client_error() || s.is_server_error() => Err(SignalWireError::Api { code: s.as_u16(), body }),
            _ => serde_json::from_str(&body).map_err(|source| SignalWireError::Decode { source, body }),
        }
    }

    /// Same as [`handle`](Self::handle) but for endpoints with no response body.
    fn handle_no_content(resp: Response) -> Result<(), SignalWireError> {
        let status = resp.status();
        if status == StatusCode::TOO_MANY_REQUESTS {
            let retry = resp
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .map(Duration::from_secs);
            return Err(SignalWireError::RateLimited(retry));
        }
        match status {
            StatusCode::UNAUTHORIZED => Err(SignalWireError::Unauthorized),
            StatusCode::NOT_FOUND => Err(SignalWireError::NotFound(resp.text()?)),
            s if s.is_client_error() || s.is_server_error() => Err(SignalWireError::Api { code: s.as_u16(), body: resp.text()? }),
            _ => Ok(()),
        }
    }

    // ---------- Auth ----------

    /// Get a JWT + refresh token (`POST /api/relay/rest/jwt`).
    pub fn get_jwt(&self) -> Result<JwtResponse, SignalWireError> {
        let resp = self.auth(self.http.post(self.url("/api/relay/rest/jwt"))).header(reqwest::header::CONTENT_LENGTH, "0").send()?;
        Self::handle(resp)
    }

    // ---------- Phone numbers ----------

    /// List phone numbers available for purchase in a given country.
    pub fn get_phone_numbers_available(&self, iso_country: &str, query: &[(String, String)]) -> Result<PhoneNumbersAvailableResponse, SignalWireError> {
        let path = format!("/api/laml/2010-04-01/Accounts/{}/AvailablePhoneNumbers/{}/Local", self.project_id, iso_country);
        let resp = self.auth(self.http.get(self.url(&path))).query(query).send()?;
        Self::handle(resp)
    }

    /// List phone numbers owned by the project.
    pub fn get_phone_numbers_owned(&self, query: &[(String, String)]) -> Result<PhoneNumbersOwnedResponse, SignalWireError> {
        let resp = self.auth(self.http.get(self.url("/api/relay/rest/phone_numbers"))).query(query).send()?;
        Self::handle(resp)
    }

    /// Buy a phone number. **This costs money.**
    pub fn buy_phone_number(&self, phone_number: &str) -> Result<BuyPhoneNumberResponse, SignalWireError> {
        let body = BuyPhoneNumberRequest { number: phone_number.to_string() };
        let resp = self.auth(self.http.post(self.url("/api/relay/rest/phone_numbers"))).json(&body).send()?;
        Self::handle(resp)
    }

    /// Update routing/configuration on an owned phone number.
    pub fn update_phone_number(&self, id: &str, request: &UpdatePhoneNumberRequest) -> Result<BuyPhoneNumberResponse, SignalWireError> {
        let path = format!("/api/relay/rest/phone_numbers/{}", id);
        let resp = self.auth(self.http.put(self.url(&path))).json(request).send()?;
        Self::handle(resp)
    }

    // ---------- SMS ----------

    /// Send an SMS message. **This costs money.**
    pub fn send_sms(&self, message: &SmsMessage) -> Result<SmsResponse, SignalWireError> {
        let path = format!("/api/laml/2010-04-01/Accounts/{}/Messages", self.project_id);
        let form = [("From", &message.from), ("To", &message.to), ("Body", &message.body)];
        let resp = self.auth(self.http.post(self.url(&path))).form(&form).send()?;
        Self::handle(resp)
    }

    /// Look up a previously-sent message by SID.
    pub fn get_message_status(&self, message_sid: &str) -> Result<SmsResponse, SignalWireError> {
        let path = format!("/api/laml/2010-04-01/Accounts/{}/Messages/{}", self.project_id, message_sid);
        let resp = self.auth(self.http.get(self.url(&path))).send()?;
        Self::handle(resp)
    }

    // ---------- Subprojects ----------

    /// List the project's subprojects.
    pub fn list_subprojects(&self, query: &[(String, String)]) -> Result<SubprojectsListResponse, SignalWireError> {
        let resp = self.auth(self.http.get(self.url("/api/laml/2010-04-01/Accounts"))).query(query).send()?;
        Self::handle(resp)
    }

    /// Fetch a single subproject by SID.
    pub fn get_subproject(&self, subproject_sid: &str) -> Result<SubprojectResponse, SignalWireError> {
        let path = format!("/api/laml/2010-04-01/Accounts/{}", subproject_sid);
        let resp = self.auth(self.http.get(self.url(&path))).send()?;
        Self::handle(resp)
    }

    /// Create a new subproject.
    pub fn create_subproject(&self, friendly_name: &str) -> Result<SubprojectResponse, SignalWireError> {
        let form = [("FriendlyName", friendly_name)];
        let resp = self.auth(self.http.post(self.url("/api/laml/2010-04-01/Accounts"))).form(&form).send()?;
        Self::handle(resp)
    }

    /// Update a subproject's friendly name and optionally its status.
    /// Pass `Some("closed")` to mark it deletable.
    pub fn update_subproject(&self, subproject_sid: &str, friendly_name: &str, status: Option<&str>) -> Result<SubprojectResponse, SignalWireError> {
        let path = format!("/api/laml/2010-04-01/Accounts/{}", subproject_sid);
        let mut form = vec![("FriendlyName", friendly_name)];
        if let Some(s) = status {
            form.push(("Status", s));
        }
        let resp = self.auth(self.http.post(self.url(&path))).form(&form).send()?;
        Self::handle(resp)
    }

    /// Delete a subproject. Must be in `closed` status first.
    pub fn delete_subproject(&self, subproject_sid: &str) -> Result<(), SignalWireError> {
        let path = format!("/api/laml/2010-04-01/Accounts/{}", subproject_sid);
        let resp = self.auth(self.http.delete(self.url(&path))).send()?;
        Self::handle_no_content(resp)
    }

    /// List phone numbers owned by a specific subproject.
    pub fn get_subproject_phone_numbers(&self, subproject_sid: &str, query: &[(String, String)]) -> Result<SubprojectPhoneNumbersResponse, SignalWireError> {
        let path = format!("/api/laml/2010-04-01/Accounts/{}/IncomingPhoneNumbers", subproject_sid);
        let resp = self.auth(self.http.get(self.url(&path))).query(query).send()?;
        Self::handle(resp)
    }

    // ---------- Lookup ----------

    /// Validate a phone number and return basic metadata.
    pub fn lookup_phone_number(&self, phone_number: &str) -> Result<PhoneLookupResponse, SignalWireError> {
        self.lookup(phone_number, LookupKind::Basic)
    }

    /// Lookup with carrier info. **Paid endpoint.**
    pub fn lookup_phone_number_with_carrier(&self, phone_number: &str) -> Result<PhoneLookupResponse, SignalWireError> {
        self.lookup(phone_number, LookupKind::Carrier)
    }

    /// Lookup with caller-name (CNAM) info. **Paid endpoint.**
    pub fn lookup_phone_number_with_caller_name(&self, phone_number: &str) -> Result<PhoneLookupResponse, SignalWireError> {
        self.lookup(phone_number, LookupKind::CallerName)
    }

    /// Generic phone-number lookup. Pick what to fetch via [`LookupKind`].
    pub fn lookup(&self, phone_number: &str, kind: LookupKind) -> Result<PhoneLookupResponse, SignalWireError> {
        let path = format!("/api/relay/rest/lookup/phone_number/{}", phone_number);
        let mut req = self.auth(self.http.get(self.url(&path)));
        if let Some((k, v)) = kind.as_query() {
            req = req.query(&[(k, v)]);
        }
        Self::handle(req.send()?)
    }
}
