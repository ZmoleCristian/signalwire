//! Request and response types, plus typed query-param builders.

use serde::{Deserialize, Serialize};

// ---------- Generic query builder ----------

/// Builder for `(String, String)` query-param tuples that reqwest's
/// [`.query()`](reqwest::RequestBuilder::query) accepts.
///
/// All the typed parameter wrappers in this module
/// ([`PhoneNumberAvailableQueryParams`], [`SubprojectQueryParams`], …)
/// build on top of this.
#[derive(Debug, Clone)]
pub struct QueryBuilder {
    params: Vec<(String, String)>,
}

impl QueryBuilder {
    /// Empty builder.
    pub fn new() -> Self {
        Self { params: Vec::new() }
    }

    /// Append a `(key, value)` pair. `value` is stringified via [`ToString`].
    pub fn push(mut self, key: impl Into<String>, value: impl ToString) -> Self {
        self.params.push((key.into(), value.to_string()));
        self
    }

    /// Consume the builder and return the underlying tuple list,
    /// ready to pass to a client method.
    pub fn build(self) -> Vec<(String, String)> {
        self.params
    }
}

// ---------- Auth ----------

/// Response from `POST /api/relay/rest/jwt` — a JWT plus a refresh token.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JwtResponse {
    /// Bearer token usable for authenticated requests.
    pub jwt_token: String,
    /// Token used to mint a fresh `jwt_token` once it expires.
    pub refresh_token: String,
}

// ---------- Available phone numbers ----------

/// Query-param builder for
/// [`get_phone_numbers_available`](crate::SignalWireClient::get_phone_numbers_available).
///
/// All filters are optional; chain them and call [`build`](Self::build).
///
/// ```
/// # use signalwire::PhoneNumberAvailableQueryParams;
/// let q = PhoneNumberAvailableQueryParams::new()
///     .area_code("206")
///     .sms_enabled(true)
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct PhoneNumberAvailableQueryParams {
    inner: QueryBuilder,
}

impl PhoneNumberAvailableQueryParams {
    /// Empty filter — returns all available numbers for the country.
    pub fn new() -> Self {
        Self { inner: QueryBuilder::new() }
    }
    /// Restrict to a specific North American area code.
    pub fn area_code(self, code: &str) -> Self {
        Self { inner: self.inner.push("AreaCode", code) }
    }
    /// Include beta numbers.
    pub fn beta(self, beta: bool) -> Self {
        Self { inner: self.inner.push("Beta", beta) }
    }
    /// Substring match on the friendly-formatted number.
    pub fn contains(self, value: &str) -> Self {
        Self {
            inner: self.inner.push("Contains", value),
        }
    }
    /// Skip numbers that require any address on file.
    pub fn exclude_all_address_required(self, value: bool) -> Self {
        Self {
            inner: self.inner.push("ExcludeAllAddressRequired", value),
        }
    }
    /// Skip numbers that require a foreign address on file.
    pub fn exclude_foreign_address_required(self, value: bool) -> Self {
        Self {
            inner: self.inner.push("ExcludeForeignAddressRequired", value),
        }
    }
    /// Skip numbers that require a local address on file.
    pub fn exclude_local_address_required(self, value: bool) -> Self {
        Self {
            inner: self.inner.push("ExcludeLocalAddressRequired", value),
        }
    }
    /// Restrict to numbers with fax capability.
    pub fn fax_enabled(self, enabled: bool) -> Self {
        Self {
            inner: self.inner.push("FaxEnabled", enabled),
        }
    }
    /// Restrict to a region (e.g. US state code).
    pub fn in_region(self, region: &str) -> Self {
        Self {
            inner: self.inner.push("InRegion", region),
        }
    }
    /// Restrict to numbers with MMS capability.
    pub fn mms_enabled(self, enabled: bool) -> Self {
        Self {
            inner: self.inner.push("MmsEnabled", enabled),
        }
    }
    /// Restrict to numbers with SMS capability.
    pub fn sms_enabled(self, enabled: bool) -> Self {
        Self {
            inner: self.inner.push("SmsEnabled", enabled),
        }
    }
    /// Restrict to numbers with voice capability.
    pub fn voice_enabled(self, enabled: bool) -> Self {
        Self {
            inner: self.inner.push("VoiceEnabled", enabled),
        }
    }
    /// Materialize into the tuple list a client method consumes.
    pub fn build(self) -> Vec<(String, String)> {
        self.inner.build()
    }
}

/// Response wrapper for available phone numbers.
#[derive(Debug, Clone, Deserialize)]
pub struct PhoneNumbersAvailableResponse {
    /// Self-link URI.
    pub uri: String,
    /// The actual list of numbers.
    #[serde(rename = "available_phone_numbers")]
    pub phone_numbers_available: Vec<PhoneNumberAvailable>,
}

/// One available phone number from the catalog.
#[derive(Debug, Clone, Deserialize)]
pub struct PhoneNumberAvailable {
    pub beta: bool,
    pub capabilities: Capabilities,
    pub friendly_name: String,
    pub iso_country: String,
    pub lata: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub phone_number: String,
    pub postal_code: Option<String>,
    pub rate_center: String,
    pub region: String,
}

/// Capabilities flags for a phone number — voice / SMS / MMS / fax.
#[derive(Debug, Clone, Deserialize)]
pub struct Capabilities {
    pub voice: Option<bool>,
    #[serde(rename = "SMS")]
    pub sms: Option<bool>,
    #[serde(rename = "MMS")]
    pub mms: Option<bool>,
    pub fax: Option<bool>,
}

// ---------- Owned phone numbers ----------

/// Filter for [`get_phone_numbers_owned`](crate::SignalWireClient::get_phone_numbers_owned)
/// and [`get_subproject_phone_numbers`](crate::SignalWireClient::get_subproject_phone_numbers).
#[derive(Debug, Clone)]
pub struct PhoneNumberOwnedFilterParams {
    inner: QueryBuilder,
}

impl PhoneNumberOwnedFilterParams {
    /// Empty filter — returns every owned number.
    pub fn new() -> Self {
        Self { inner: QueryBuilder::new() }
    }
    /// Substring match on the number's friendly name.
    pub fn filter_name(self, name: &str) -> Self {
        Self {
            inner: self.inner.push("filter_name", name),
        }
    }
    /// Substring match on the actual phone number.
    pub fn filter_number(self, number: &str) -> Self {
        Self {
            inner: self.inner.push("filter_number", number),
        }
    }
    /// Materialize into the tuple list a client method consumes.
    pub fn build(self) -> Vec<(String, String)> {
        self.inner.build()
    }
}

/// Paginated response wrapping a list of [`OwnedPhoneNumber`].
#[derive(Debug, Clone, Deserialize)]
pub struct PhoneNumbersOwnedResponse {
    /// Pagination links.
    pub links: Links,
    /// Page of owned phone numbers.
    pub data: Vec<OwnedPhoneNumber>,
}

/// HATEOAS pagination links.
#[derive(Debug, Clone, Deserialize)]
pub struct Links {
    /// Current page URL.
    #[serde(rename = "self")]
    pub self_field: String,
    /// First page URL.
    pub first: String,
    /// Next page URL, if any.
    pub next: Option<String>,
    /// Previous page URL, if any.
    pub prev: Option<String>,
}

/// One phone number owned by the project (or a subproject), with all
/// the call/message routing knobs SignalWire exposes.
///
/// Most fields are `Option<String>` because SignalWire only returns the
/// ones you've configured.
#[derive(Debug, Clone, Deserialize)]
pub struct OwnedPhoneNumber {
    pub id: String,
    pub number: String,
    pub name: Option<String>,
    pub call_handler: Option<String>,
    pub call_receive_mode: Option<String>,
    pub call_request_url: Option<String>,
    pub call_request_method: Option<String>,
    pub call_fallback_url: Option<String>,
    pub call_fallback_method: Option<String>,
    pub call_status_callback_url: Option<String>,
    pub call_status_callback_method: Option<String>,
    pub call_laml_application_id: Option<String>,
    pub call_dialogflow_agent_id: Option<String>,
    pub call_relay_topic: Option<String>,
    pub call_relay_topic_status_callback_url: Option<String>,
    pub call_relay_context: Option<String>,
    pub call_relay_context_status_callback_url: Option<String>,
    pub call_relay_application: Option<String>,
    pub call_relay_connector_id: Option<String>,
    pub call_sip_endpoint_id: Option<String>,
    pub call_verto_resource: Option<String>,
    pub call_video_room_id: Option<String>,
    pub message_handler: Option<String>,
    pub message_request_url: Option<String>,
    pub message_request_method: Option<String>,
    pub message_fallback_url: Option<String>,
    pub message_fallback_method: Option<String>,
    pub message_laml_application_id: Option<String>,
    pub message_relay_topic: Option<String>,
    pub message_relay_context: Option<String>,
    pub message_relay_application: Option<String>,
    pub capabilities: Vec<String>,
    pub number_type: Option<String>,
    pub e911_address_id: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub next_billed_at: Option<String>,
}

// ---------- Buy phone number ----------

/// Body for the buy-phone-number request.
#[derive(Debug, Clone, Serialize)]
pub struct BuyPhoneNumberRequest {
    /// E.164 phone number to purchase.
    pub number: String,
}

/// Buying a number returns the same shape as listing owned numbers.
pub type BuyPhoneNumberResponse = OwnedPhoneNumber;

// ---------- Update phone number ----------

/// Body for [`update_phone_number`](crate::SignalWireClient::update_phone_number).
///
/// All fields are `Option`. Anything left `None` is omitted from the request
/// (via `serde(skip_serializing_if = "Option::is_none")`) and therefore left
/// unchanged on the server side.
///
/// Construct an empty request with [`Self::new`] and assign the fields you
/// want to change directly.
#[derive(Debug, Clone, Serialize)]
pub struct UpdatePhoneNumberRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_handler: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_receive_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_request_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_request_method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_fallback_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_fallback_method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_status_callback_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_status_callback_method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_laml_application_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_dialogflow_agent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_relay_topic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_relay_topic_status_callback_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_relay_script_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_relay_application: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_relay_connector_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_sip_endpoint_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_verto_resource: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_video_room_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_handler: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_request_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_request_method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_fallback_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_fallback_method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_laml_application_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_relay_topic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_relay_application: Option<String>,
}

impl UpdatePhoneNumberRequest {
    /// Empty patch — every field starts `None`. Mutate the fields you want
    /// to change before passing this to the client.
    pub fn new() -> Self {
        Self {
            name: None,
            call_handler: None,
            call_receive_mode: None,
            call_request_url: None,
            call_request_method: None,
            call_fallback_url: None,
            call_fallback_method: None,
            call_status_callback_url: None,
            call_status_callback_method: None,
            call_laml_application_id: None,
            call_dialogflow_agent_id: None,
            call_relay_topic: None,
            call_relay_topic_status_callback_url: None,
            call_relay_script_url: None,
            call_relay_application: None,
            call_relay_connector_id: None,
            call_sip_endpoint_id: None,
            call_verto_resource: None,
            call_video_room_id: None,
            message_handler: None,
            message_request_url: None,
            message_request_method: None,
            message_fallback_url: None,
            message_fallback_method: None,
            message_laml_application_id: None,
            message_relay_topic: None,
            message_relay_application: None,
        }
    }
}

// ---------- SMS ----------

/// Outgoing SMS payload for [`send_sms`](crate::SignalWireClient::send_sms).
#[derive(Debug, Clone, Serialize)]
pub struct SmsMessage {
    /// Body text.
    pub body: String,
    /// Sender — must be a number owned by the project.
    pub from: String,
    /// Recipient in E.164 format (`+1...`).
    pub to: String,
}

/// Response from `send_sms` and `get_message_status` — a Twilio-style
/// LaML message resource.
#[derive(Debug, Clone, Deserialize)]
pub struct SmsResponse {
    pub sid: String,
    pub date_created: String,
    pub date_updated: String,
    pub date_sent: Option<String>,
    pub account_sid: String,
    pub to: String,
    pub from: String,
    pub messaging_service_sid: Option<String>,
    pub body: String,
    pub status: String,
    pub num_segments: i32,
    pub num_media: i32,
    pub direction: String,
    pub api_version: String,
    pub price: Option<f64>,
    pub price_unit: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub uri: String,
    pub subresource_uris: Option<SubresourceUris>,
}

impl SmsResponse {
    /// Parse the raw `status` string into a [`MessageStatus`] enum.
    pub fn get_status(&self) -> MessageStatus {
        MessageStatus::from(self.status.as_str())
    }
}

/// Sub-resource URIs attached to an SMS response (currently only `media`).
#[derive(Debug, Clone, Deserialize)]
pub struct SubresourceUris {
    /// URI to the message's media (MMS attachments).
    pub media: Option<String>,
}

/// Lifecycle states an SMS message can be in.
///
/// Returned by [`SmsResponse::get_status`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageStatus {
    /// Accepted, waiting to dispatch.
    Queued,
    /// Currently being sent to the carrier.
    Sending,
    /// Handed off to the carrier.
    Sent,
    /// Carrier confirmed delivery.
    Delivered,
    /// Carrier rejected the message.
    Failed,
    /// Carrier accepted but couldn't deliver.
    Undelivered,
    /// Status string didn't match anything we know.
    Unknown,
}

impl From<&str> for MessageStatus {
    fn from(status: &str) -> Self {
        match status.to_lowercase().as_str() {
            "queued" => Self::Queued,
            "sending" => Self::Sending,
            "sent" => Self::Sent,
            "delivered" => Self::Delivered,
            "failed" => Self::Failed,
            "undelivered" => Self::Undelivered,
            _ => Self::Unknown,
        }
    }
}

impl std::fmt::Display for MessageStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Queued => "queued",
            Self::Sending => "sending",
            Self::Sent => "sent",
            Self::Delivered => "delivered",
            Self::Failed => "failed",
            Self::Undelivered => "undelivered",
            Self::Unknown => "unknown",
        };
        f.write_str(s)
    }
}

// ---------- Subprojects ----------

/// A subproject (LaML "Account" resource).
///
/// Returned by [`get_subproject`](crate::SignalWireClient::get_subproject),
/// [`create_subproject`](crate::SignalWireClient::create_subproject), and
/// [`update_subproject`](crate::SignalWireClient::update_subproject).
#[derive(Debug, Clone, Deserialize)]
pub struct SubprojectResponse {
    pub sid: String,
    pub friendly_name: String,
    pub status: String,
    pub auth_token: String,
    pub date_created: String,
    pub date_updated: String,
    #[serde(rename = "type")]
    pub account_type: Option<String>,
    pub owner_account_sid: Option<String>,
    pub uri: Option<String>,
    pub subproject: Option<bool>,
    pub signing_key: Option<String>,
    pub subresource_uris: SubprojectResourceUris,
}

/// Sub-resource URIs attached to a subproject (calls, messages, recordings, …).
#[derive(Debug, Clone, Deserialize)]
pub struct SubprojectResourceUris {
    pub addresses: Option<String>,
    pub available_phone_numbers: Option<String>,
    pub applications: Option<String>,
    pub authorized_connect_apps: Option<String>,
    pub calls: Option<String>,
    pub conferences: Option<String>,
    pub connect_apps: Option<String>,
    pub incoming_phone_numbers: Option<String>,
    pub keys: Option<String>,
    pub notifications: Option<String>,
    pub outgoing_caller_ids: Option<String>,
    pub queues: Option<String>,
    pub recordings: Option<String>,
    pub sandbox: Option<String>,
    pub sip: Option<String>,
    pub short_codes: Option<String>,
    pub messages: Option<String>,
    pub transcriptions: Option<String>,
    pub usage: Option<String>,
}

/// Paginated list of [`SubprojectResponse`].
///
/// The first entry is always the main project itself.
#[derive(Debug, Clone, Deserialize)]
pub struct SubprojectsListResponse {
    pub uri: Option<String>,
    pub first_page_uri: String,
    pub next_page_uri: Option<String>,
    pub previous_page_uri: Option<String>,
    pub page: Option<i32>,
    pub page_size: Option<i32>,
    pub accounts: Vec<SubprojectResponse>,
}

/// Filter for [`list_subprojects`](crate::SignalWireClient::list_subprojects).
#[derive(Debug, Clone)]
pub struct SubprojectQueryParams {
    inner: QueryBuilder,
}

impl SubprojectQueryParams {
    /// Empty filter — returns all subprojects.
    pub fn new() -> Self {
        Self { inner: QueryBuilder::new() }
    }
    /// Exact-match on friendly name.
    pub fn friendly_name(self, friendly_name: &str) -> Self {
        Self {
            inner: self.inner.push("FriendlyName", friendly_name),
        }
    }
    /// Status filter — `"active"`, `"suspended"`, or `"closed"`.
    pub fn status(self, status: &str) -> Self {
        Self { inner: self.inner.push("Status", status) }
    }
    /// Materialize into the tuple list a client method consumes.
    pub fn build(self) -> Vec<(String, String)> {
        self.inner.build()
    }
}

/// Paginated list of phone numbers belonging to a subproject.
#[derive(Debug, Clone, Deserialize)]
pub struct SubprojectPhoneNumbersResponse {
    pub uri: String,
    pub first_page_uri: String,
    pub next_page_uri: Option<String>,
    pub previous_page_uri: Option<String>,
    pub page: i32,
    pub page_size: i32,
    pub incoming_phone_numbers: Vec<SubprojectPhoneNumber>,
}

/// A phone number resource as returned under a subproject's
/// `IncomingPhoneNumbers` collection.
#[derive(Debug, Clone, Deserialize)]
pub struct SubprojectPhoneNumber {
    pub sid: String,
    pub account_sid: String,
    pub friendly_name: String,
    pub phone_number: String,
    pub voice_url: Option<String>,
    pub voice_method: Option<String>,
    pub voice_fallback_url: Option<String>,
    pub voice_fallback_method: Option<String>,
    pub status_callback: Option<String>,
    pub status_callback_method: Option<String>,
    pub voice_caller_id_lookup: Option<bool>,
    pub voice_application_sid: Option<String>,
    pub date_created: String,
    pub date_updated: String,
    pub sms_url: Option<String>,
    pub sms_method: Option<String>,
    pub sms_fallback_url: Option<String>,
    pub sms_fallback_method: Option<String>,
    pub sms_application_sid: Option<String>,
    pub capabilities: PhoneNumberCapabilities,
    pub beta: bool,
    pub uri: String,
    pub trunk_sid: Option<String>,
    pub emergency_status: Option<String>,
    pub emergency_address_sid: Option<String>,
    pub emergency_address_status: Option<String>,
    pub status: Option<String>,
}

/// Capabilities for an incoming phone number, all booleans.
#[derive(Debug, Clone, Deserialize)]
pub struct PhoneNumberCapabilities {
    pub voice: bool,
    pub sms: bool,
    pub mms: bool,
    pub fax: bool,
}

// ---------- Lookup & validation ----------

/// Response from the phone-lookup endpoint.
///
/// Most fields are `Option` because what's populated depends on the
/// [`LookupKind`] you asked for. `carrier` and `caller_name` are only
/// non-`None` when you request the matching paid lookup.
#[derive(Debug, Clone, Deserialize)]
pub struct PhoneLookupResponse {
    pub country_code_number: Option<i32>,
    pub national_number: Option<String>,
    pub possible_number: Option<bool>,
    pub valid_number: Option<bool>,
    pub national_number_formatted: Option<String>,
    pub international_number_formatted: Option<String>,
    pub e164: Option<String>,
    pub location: Option<String>,
    pub country_code: String,
    pub timezones: Option<Vec<String>>,
    pub number_type: Option<String>,
    pub carrier: Option<CarrierInfo>,
    pub caller_name: Option<CallerNameInfo>,
}

/// Carrier metadata for a phone number — populated only on
/// [`LookupKind::Carrier`] requests.
#[derive(Debug, Clone, Deserialize)]
pub struct CarrierInfo {
    pub mobile_country_code: Option<String>,
    pub mobile_network_code: Option<String>,
    pub name: Option<String>,
    /// `mobile`, `landline`, `voip`, …
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub error_code: Option<String>,
}

/// CNAM (caller-name) metadata — populated only on
/// [`LookupKind::CallerName`] requests.
#[derive(Debug, Clone, Deserialize)]
pub struct CallerNameInfo {
    pub caller_name: Option<String>,
    pub caller_type: Option<String>,
    pub error_code: Option<String>,
}

/// What additional info to fetch from the lookup endpoint.
///
/// Picked by [`SignalWireClient::lookup`](crate::SignalWireClient::lookup).
#[derive(Debug, Clone, Copy)]
pub enum LookupKind {
    /// Validation, formatting, country/region. Free.
    Basic,
    /// Carrier info (`type=carrier`). **Paid.**
    Carrier,
    /// Caller-name (CNAM) lookup (`type=caller-name`). **Paid.**
    CallerName,
}

impl LookupKind {
    pub(crate) fn as_query(self) -> Option<(&'static str, &'static str)> {
        match self {
            Self::Basic => None,
            Self::Carrier => Some(("Type", "carrier")),
            Self::CallerName => Some(("Type", "caller-name")),
        }
    }
}
