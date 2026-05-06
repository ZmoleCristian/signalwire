//! Async [`SignalWireClient`] — built on `reqwest` + `rustls-tls`.

use std::time::Duration;

use reqwest::{Client as HttpClient, Response, StatusCode};
use serde::de::DeserializeOwned;

use crate::{errors::SignalWireError, types::*};

/// Default per-request timeout applied to the bundled HTTP client.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Async client for the SignalWire REST API.
///
/// Cheap to clone — the inner `reqwest::Client` shares its connection pool
/// across clones, so prefer cloning over rebuilding.
///
/// # Construction
///
/// ```no_run
/// use signalwire::SignalWireClient;
/// # fn main() -> Result<(), signalwire::SignalWireError> {
/// let client = SignalWireClient::new("space", "project_id", "api_key")?;
/// # Ok(()) }
/// ```
#[derive(Debug, Clone)]
pub struct SignalWireClient {
    space_name: String,
    project_id: String,
    api_key: String,
    http: HttpClient,
}

impl SignalWireClient {
    /// Build a client with the default `reqwest` HTTP client (30s timeout, rustls-tls).
    ///
    /// Returns `Err(SignalWireError::Http)` if the underlying TLS backend
    /// fails to initialize.
    pub fn new(space_name: impl Into<String>, project_id: impl Into<String>, api_key: impl Into<String>) -> Result<Self, SignalWireError> {
        let http = HttpClient::builder().timeout(DEFAULT_TIMEOUT).build()?;
        Ok(Self {
            space_name: space_name.into(),
            project_id: project_id.into(),
            api_key: api_key.into(),
            http,
        })
    }

    /// Swap in a custom-configured `reqwest::Client` (proxies, headers, custom TLS, etc.).
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

    fn auth(&self, b: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        b.basic_auth(&self.project_id, Some(&self.api_key))
    }

    /// Translate a JSON response into `T` or the right `SignalWireError` variant.
    async fn handle<T: DeserializeOwned>(resp: Response) -> Result<T, SignalWireError> {
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

        let body = resp.text().await?;

        match status {
            StatusCode::UNAUTHORIZED => Err(SignalWireError::Unauthorized),
            StatusCode::NOT_FOUND => Err(SignalWireError::NotFound(body)),
            s if s.is_client_error() || s.is_server_error() => Err(SignalWireError::Api { code: s.as_u16(), body }),
            _ => serde_json::from_str(&body).map_err(|source| SignalWireError::Decode { source, body }),
        }
    }

    /// Same as [`handle`](Self::handle) but for endpoints that return no body on success.
    async fn handle_no_content(resp: Response) -> Result<(), SignalWireError> {
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
            StatusCode::NOT_FOUND => Err(SignalWireError::NotFound(resp.text().await?)),
            s if s.is_client_error() || s.is_server_error() => Err(SignalWireError::Api {
                code: s.as_u16(),
                body: resp.text().await?,
            }),
            _ => Ok(()),
        }
    }

    // ---------- Auth ----------

    /// Get a JWT + refresh token (`POST /api/relay/rest/jwt`).
    pub async fn get_jwt(&self) -> Result<JwtResponse, SignalWireError> {
        let resp = self.auth(self.http.post(self.url("/api/relay/rest/jwt"))).header(reqwest::header::CONTENT_LENGTH, "0").send().await?;
        Self::handle(resp).await
    }

    // ---------- Phone numbers ----------

    /// List phone numbers available for purchase in a given country.
    ///
    /// Use [`PhoneNumberAvailableQueryParams`] to build the query.
    /// SignalWire's currently-supported `iso_country` is `"US"`.
    pub async fn get_phone_numbers_available(&self, iso_country: &str, query: &[(String, String)]) -> Result<PhoneNumbersAvailableResponse, SignalWireError> {
        let path = format!("/api/laml/2010-04-01/Accounts/{}/AvailablePhoneNumbers/{}/Local", self.project_id, iso_country);
        let resp = self.auth(self.http.get(self.url(&path))).query(query).send().await?;
        Self::handle(resp).await
    }

    /// List phone numbers owned by the project.
    ///
    /// Use [`PhoneNumberOwnedFilterParams`] to filter by name or number.
    pub async fn get_phone_numbers_owned(&self, query: &[(String, String)]) -> Result<PhoneNumbersOwnedResponse, SignalWireError> {
        let resp = self.auth(self.http.get(self.url("/api/relay/rest/phone_numbers"))).query(query).send().await?;
        Self::handle(resp).await
    }

    /// Buy a phone number listed by [`get_phone_numbers_available`](Self::get_phone_numbers_available).
    ///
    /// **This costs money.**
    pub async fn buy_phone_number(&self, phone_number: &str) -> Result<BuyPhoneNumberResponse, SignalWireError> {
        let body = BuyPhoneNumberRequest { number: phone_number.to_string() };
        let resp = self.auth(self.http.post(self.url("/api/relay/rest/phone_numbers"))).json(&body).send().await?;
        Self::handle(resp).await
    }

    /// Update routing/configuration on an owned phone number.
    ///
    /// `id` is the resource ID returned in [`OwnedPhoneNumber::id`].
    /// Any field left `None` in `request` is omitted from the body and the
    /// server keeps its current value.
    pub async fn update_phone_number(&self, id: &str, request: &UpdatePhoneNumberRequest) -> Result<BuyPhoneNumberResponse, SignalWireError> {
        let path = format!("/api/relay/rest/phone_numbers/{}", id);
        let resp = self.auth(self.http.put(self.url(&path))).json(request).send().await?;
        Self::handle(resp).await
    }

    // ---------- SMS ----------

    /// Send an SMS message.
    ///
    /// **This costs money.** `from` must be a number owned by the project (or
    /// a messaging-service alias).
    pub async fn send_sms(&self, message: &SmsMessage) -> Result<SmsResponse, SignalWireError> {
        let path = format!("/api/laml/2010-04-01/Accounts/{}/Messages", self.project_id);
        let form = [("From", &message.from), ("To", &message.to), ("Body", &message.body)];
        let resp = self.auth(self.http.post(self.url(&path))).form(&form).send().await?;
        Self::handle(resp).await
    }

    /// Look up a previously-sent message by SID.
    ///
    /// Use [`SmsResponse::get_status`] for an enum view of the status string.
    pub async fn get_message_status(&self, message_sid: &str) -> Result<SmsResponse, SignalWireError> {
        let path = format!("/api/laml/2010-04-01/Accounts/{}/Messages/{}", self.project_id, message_sid);
        let resp = self.auth(self.http.get(self.url(&path))).send().await?;
        Self::handle(resp).await
    }

    // ---------- Subprojects ----------

    /// List the project's subprojects (sub-accounts).
    ///
    /// The response always contains the main project as the first entry.
    pub async fn list_subprojects(&self, query: &[(String, String)]) -> Result<SubprojectsListResponse, SignalWireError> {
        let resp = self.auth(self.http.get(self.url("/api/laml/2010-04-01/Accounts"))).query(query).send().await?;
        Self::handle(resp).await
    }

    /// Fetch a single subproject by SID.
    pub async fn get_subproject(&self, subproject_sid: &str) -> Result<SubprojectResponse, SignalWireError> {
        let path = format!("/api/laml/2010-04-01/Accounts/{}", subproject_sid);
        let resp = self.auth(self.http.get(self.url(&path))).send().await?;
        Self::handle(resp).await
    }

    /// Create a new subproject.
    pub async fn create_subproject(&self, friendly_name: &str) -> Result<SubprojectResponse, SignalWireError> {
        let form = [("FriendlyName", friendly_name)];
        let resp = self.auth(self.http.post(self.url("/api/laml/2010-04-01/Accounts"))).form(&form).send().await?;
        Self::handle(resp).await
    }

    /// Update a subproject's friendly name and optionally its status
    /// (`"active"`, `"suspended"`, or `"closed"`).
    ///
    /// SignalWire requires `Status=closed` before [`delete_subproject`](Self::delete_subproject)
    /// will accept the deletion.
    pub async fn update_subproject(&self, subproject_sid: &str, friendly_name: &str, status: Option<&str>) -> Result<SubprojectResponse, SignalWireError> {
        let path = format!("/api/laml/2010-04-01/Accounts/{}", subproject_sid);
        let mut form = vec![("FriendlyName", friendly_name)];
        if let Some(s) = status {
            form.push(("Status", s));
        }
        let resp = self.auth(self.http.post(self.url(&path))).form(&form).send().await?;
        Self::handle(resp).await
    }

    /// Delete a subproject. Must be in `closed` status first
    /// (see [`update_subproject`](Self::update_subproject)).
    pub async fn delete_subproject(&self, subproject_sid: &str) -> Result<(), SignalWireError> {
        let path = format!("/api/laml/2010-04-01/Accounts/{}", subproject_sid);
        let resp = self.auth(self.http.delete(self.url(&path))).send().await?;
        Self::handle_no_content(resp).await
    }

    /// List phone numbers owned by a specific subproject.
    pub async fn get_subproject_phone_numbers(&self, subproject_sid: &str, query: &[(String, String)]) -> Result<SubprojectPhoneNumbersResponse, SignalWireError> {
        let path = format!("/api/laml/2010-04-01/Accounts/{}/IncomingPhoneNumbers", subproject_sid);
        let resp = self.auth(self.http.get(self.url(&path))).query(query).send().await?;
        Self::handle(resp).await
    }

    // ---------- Lookup ----------

    /// Validate a phone number and return basic metadata
    /// (country code, formatting, location, timezone).
    ///
    /// Equivalent to `lookup(num, LookupKind::Basic)`.
    pub async fn lookup_phone_number(&self, phone_number: &str) -> Result<PhoneLookupResponse, SignalWireError> {
        self.lookup(phone_number, LookupKind::Basic).await
    }

    /// Lookup with carrier info (`type=carrier`). **Paid endpoint.**
    pub async fn lookup_phone_number_with_carrier(&self, phone_number: &str) -> Result<PhoneLookupResponse, SignalWireError> {
        self.lookup(phone_number, LookupKind::Carrier).await
    }

    /// Lookup with caller-name (CNAM) info (`type=caller-name`). **Paid endpoint.**
    pub async fn lookup_phone_number_with_caller_name(&self, phone_number: &str) -> Result<PhoneLookupResponse, SignalWireError> {
        self.lookup(phone_number, LookupKind::CallerName).await
    }

    /// Generic phone-number lookup. Pick what to fetch via [`LookupKind`].
    ///
    /// `Carrier` and `CallerName` lookups are billed; `Basic` is free.
    pub async fn lookup(&self, phone_number: &str, kind: LookupKind) -> Result<PhoneLookupResponse, SignalWireError> {
        let path = format!("/api/relay/rest/lookup/phone_number/{}", phone_number);
        let mut req = self.auth(self.http.get(self.url(&path)));
        if let Some((k, v)) = kind.as_query() {
            req = req.query(&[(k, v)]);
        }
        Self::handle(req.send().await?).await
    }
}
