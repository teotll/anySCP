use std::{fmt, sync::Arc, time::Duration};

use reqwest::{redirect::Policy, Client, Method, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::State;
use tracing::instrument;

use crate::db::HostDb;

const CLOUDFLARE_API_BASE: &str = "https://api.cloudflare.com/client/v4";
const MAX_JSON_BODY_BYTES: usize = 64 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum R2Error {
    #[error("R2 connection not found: {0}")]
    ConnectionNotFound(String),
    #[error("Connection is not a Cloudflare R2 connection")]
    NotR2Connection,
    #[error("Missing Cloudflare account ID for this R2 connection")]
    MissingAccountId,
    #[error("Missing Cloudflare API token for this R2 connection")]
    MissingApiToken,
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    #[error("Network error: {0}")]
    Network(String),
    #[error("Cloudflare API error: {message}")]
    Api { code: Option<i64>, message: String },
    #[error("Could not decode Cloudflare response: {0}")]
    Decode(String),
    #[error("I/O error: {0}")]
    Io(String),
}

impl Serialize for R2Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let field_count = if matches!(self, R2Error::Api { .. }) {
            3
        } else {
            2
        };
        let mut state = serializer.serialize_struct("R2Error", field_count)?;
        let kind = match self {
            R2Error::ConnectionNotFound(_) => "connection_not_found",
            R2Error::NotR2Connection => "not_r2_connection",
            R2Error::MissingAccountId => "missing_account_id",
            R2Error::MissingApiToken => "missing_api_token",
            R2Error::InvalidRequest(_) => "invalid_request",
            R2Error::Network(_) => "network",
            R2Error::Api { .. } => "cloudflare_api",
            R2Error::Decode(_) => "decode",
            R2Error::Io(_) => "io",
        };
        state.serialize_field("kind", kind)?;
        state.serialize_field("message", &self.to_string())?;
        if let R2Error::Api { code, .. } = self {
            state.serialize_field("code", code)?;
        }
        state.end()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R2Bucket {
    pub name: Option<String>,
    pub creation_date: Option<String>,
    pub jurisdiction: Option<String>,
    pub location: Option<String>,
    pub storage_class: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct R2CreateBucketRequest {
    pub name: String,
    pub jurisdiction: Option<String>,
    pub location_hint: Option<String>,
    pub storage_class: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct R2PatchBucketRequest {
    pub storage_class: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct R2AttachCustomDomainRequest {
    pub domain: String,
    pub zone_id: String,
    pub enabled: Option<bool>,
    #[serde(alias = "minTLS")]
    pub min_tls: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CloudflareEnvelope<T> {
    success: bool,
    #[serde(default)]
    errors: Vec<Value>,
    #[serde(default)]
    messages: Vec<Value>,
    result: Option<T>,
}

struct R2Auth {
    account_id: String,
    api_token: String,
}

impl fmt::Debug for R2Auth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("R2Auth")
            .field("account_id", &self.account_id)
            .field("api_token", &"<redacted>")
            .finish()
    }
}

pub struct R2Manager {
    http: Client,
}

impl R2Manager {
    pub fn new() -> Result<Self, R2Error> {
        let http = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .redirect(Policy::none())
            .https_only(true)
            .build()
            .map_err(|e| R2Error::Network(e.to_string()))?;
        Ok(Self { http })
    }
}

#[derive(Clone)]
struct R2Client {
    http: Client,
    account_id: String,
    api_token: String,
}

impl R2Client {
    async fn request<T>(
        &self,
        method: Method,
        path: &str,
        jurisdiction: Option<&str>,
        body: Option<Value>,
    ) -> Result<T, R2Error>
    where
        T: for<'de> Deserialize<'de>,
    {
        let url = format!("{CLOUDFLARE_API_BASE}/accounts/{}{}", self.account_id, path);
        let mut request = self.http.request(method, url).bearer_auth(&self.api_token);

        if let Some(jurisdiction) = jurisdiction.filter(|value| !value.trim().is_empty()) {
            request = request.header("cf-r2-jurisdiction", jurisdiction);
        }

        if let Some(body) = body {
            request = request.json(&body);
        }

        let envelope: CloudflareEnvelope<T> = self.execute(request).await?;
        envelope.result.ok_or_else(|| {
            R2Error::Decode("Cloudflare response did not include a result".to_string())
        })
    }

    async fn request_value(
        &self,
        method: Method,
        path: &str,
        jurisdiction: Option<&str>,
        body: Option<Value>,
    ) -> Result<Value, R2Error> {
        let url = format!("{CLOUDFLARE_API_BASE}/accounts/{}{}", self.account_id, path);
        let mut request = self.http.request(method, url).bearer_auth(&self.api_token);

        if let Some(jurisdiction) = jurisdiction.filter(|value| !value.trim().is_empty()) {
            request = request.header("cf-r2-jurisdiction", jurisdiction);
        }

        if let Some(body) = body {
            request = request.json(&body);
        }

        let envelope: CloudflareEnvelope<Value> = self.execute(request).await?;
        Ok(envelope.result.unwrap_or(Value::Null))
    }

    async fn execute<T>(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<CloudflareEnvelope<T>, R2Error>
    where
        T: for<'de> Deserialize<'de>,
    {
        let response = request
            .send()
            .await
            .map_err(|e| R2Error::Network(e.to_string()))?;
        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| R2Error::Network(e.to_string()))?;

        let envelope: CloudflareEnvelope<T> =
            serde_json::from_str(&text).map_err(|e| decode_error(status, &text, &e))?;

        if !status.is_success() || !envelope.success {
            return Err(cloudflare_error(&envelope));
        }

        Ok(envelope)
    }
}

#[derive(Debug, Deserialize)]
struct BucketListResponse {
    buckets: Option<Vec<R2Bucket>>,
}

#[tauri::command]
#[instrument(skip(db, r2_manager))]
pub async fn r2_list_buckets(
    connection_id: String,
    db: State<'_, Arc<HostDb>>,
    r2_manager: State<'_, Arc<R2Manager>>,
) -> Result<Vec<R2Bucket>, R2Error> {
    let client = client_for_connection(connection_id, db, r2_manager.http.clone()).await?;
    let result: BucketListResponse = client
        .request(Method::GET, "/r2/buckets", None, None)
        .await?;
    Ok(result.buckets.unwrap_or_default())
}

#[tauri::command]
#[instrument(skip(db, r2_manager))]
pub async fn r2_get_bucket(
    connection_id: String,
    bucket_name: String,
    jurisdiction: Option<String>,
    db: State<'_, Arc<HostDb>>,
    r2_manager: State<'_, Arc<R2Manager>>,
) -> Result<R2Bucket, R2Error> {
    validate_path_segment(&bucket_name, "bucket name")?;
    let client = client_for_connection(connection_id, db, r2_manager.http.clone()).await?;
    client
        .request(
            Method::GET,
            &format!("/r2/buckets/{bucket_name}"),
            jurisdiction.as_deref(),
            None,
        )
        .await
}

#[tauri::command]
#[instrument(skip(db, r2_manager))]
pub async fn r2_create_bucket(
    connection_id: String,
    request: R2CreateBucketRequest,
    db: State<'_, Arc<HostDb>>,
    r2_manager: State<'_, Arc<R2Manager>>,
) -> Result<R2Bucket, R2Error> {
    validate_path_segment(&request.name, "bucket name")?;
    let client = client_for_connection(connection_id, db, r2_manager.http.clone()).await?;
    let mut body = json!({ "name": request.name });
    if let Some(value) = request
        .location_hint
        .filter(|value| !value.trim().is_empty())
    {
        body["locationHint"] = json!(value);
    }
    if let Some(value) = request
        .storage_class
        .filter(|value| !value.trim().is_empty())
    {
        body["storageClass"] = json!(value);
    }

    client
        .request(
            Method::POST,
            "/r2/buckets",
            request.jurisdiction.as_deref(),
            Some(body),
        )
        .await
}

#[tauri::command]
#[instrument(skip(db, r2_manager))]
pub async fn r2_patch_bucket(
    connection_id: String,
    bucket_name: String,
    jurisdiction: Option<String>,
    request: R2PatchBucketRequest,
    db: State<'_, Arc<HostDb>>,
    r2_manager: State<'_, Arc<R2Manager>>,
) -> Result<R2Bucket, R2Error> {
    validate_path_segment(&bucket_name, "bucket name")?;
    let client = client_for_connection(connection_id, db, r2_manager.http.clone()).await?;
    let mut body = Value::Object(Default::default());
    if let Some(value) = request
        .storage_class
        .filter(|value| !value.trim().is_empty())
    {
        body["storageClass"] = json!(value);
    }

    client
        .request(
            Method::PATCH,
            &format!("/r2/buckets/{bucket_name}"),
            jurisdiction.as_deref(),
            Some(body),
        )
        .await
}

#[tauri::command]
#[instrument(skip(db, r2_manager))]
pub async fn r2_delete_bucket(
    connection_id: String,
    bucket_name: String,
    confirm_name: String,
    jurisdiction: Option<String>,
    db: State<'_, Arc<HostDb>>,
    r2_manager: State<'_, Arc<R2Manager>>,
) -> Result<Value, R2Error> {
    validate_path_segment(&bucket_name, "bucket name")?;
    if bucket_name != confirm_name {
        return Err(R2Error::InvalidRequest(
            "confirmation does not match bucket name".to_string(),
        ));
    }

    let client = client_for_connection(connection_id, db, r2_manager.http.clone()).await?;
    client
        .request_value(
            Method::DELETE,
            &format!("/r2/buckets/{bucket_name}"),
            jurisdiction.as_deref(),
            None,
        )
        .await
}

#[tauri::command]
#[instrument(skip(db, r2_manager))]
pub async fn r2_get_cors(
    connection_id: String,
    bucket_name: String,
    jurisdiction: Option<String>,
    db: State<'_, Arc<HostDb>>,
    r2_manager: State<'_, Arc<R2Manager>>,
) -> Result<Value, R2Error> {
    bucket_json_command(
        connection_id,
        bucket_name,
        jurisdiction,
        db,
        r2_manager.http.clone(),
        Method::GET,
        "/cors",
        None,
    )
    .await
}

#[tauri::command]
#[instrument(skip(db, r2_manager, policy))]
pub async fn r2_put_cors(
    connection_id: String,
    bucket_name: String,
    jurisdiction: Option<String>,
    policy: Value,
    db: State<'_, Arc<HostDb>>,
    r2_manager: State<'_, Arc<R2Manager>>,
) -> Result<Value, R2Error> {
    validate_json_object(&policy, "CORS policy")?;
    bucket_json_command(
        connection_id,
        bucket_name,
        jurisdiction,
        db,
        r2_manager.http.clone(),
        Method::PUT,
        "/cors",
        Some(policy),
    )
    .await
}

#[tauri::command]
#[instrument(skip(db, r2_manager))]
pub async fn r2_delete_cors(
    connection_id: String,
    bucket_name: String,
    jurisdiction: Option<String>,
    db: State<'_, Arc<HostDb>>,
    r2_manager: State<'_, Arc<R2Manager>>,
) -> Result<Value, R2Error> {
    bucket_json_command(
        connection_id,
        bucket_name,
        jurisdiction,
        db,
        r2_manager.http.clone(),
        Method::DELETE,
        "/cors",
        None,
    )
    .await
}

#[tauri::command]
#[instrument(skip(db, r2_manager))]
pub async fn r2_get_lifecycle(
    connection_id: String,
    bucket_name: String,
    jurisdiction: Option<String>,
    db: State<'_, Arc<HostDb>>,
    r2_manager: State<'_, Arc<R2Manager>>,
) -> Result<Value, R2Error> {
    bucket_json_command(
        connection_id,
        bucket_name,
        jurisdiction,
        db,
        r2_manager.http.clone(),
        Method::GET,
        "/lifecycle",
        None,
    )
    .await
}

#[tauri::command]
#[instrument(skip(db, r2_manager, policy))]
pub async fn r2_put_lifecycle(
    connection_id: String,
    bucket_name: String,
    jurisdiction: Option<String>,
    policy: Value,
    db: State<'_, Arc<HostDb>>,
    r2_manager: State<'_, Arc<R2Manager>>,
) -> Result<Value, R2Error> {
    validate_json_object(&policy, "lifecycle policy")?;
    bucket_json_command(
        connection_id,
        bucket_name,
        jurisdiction,
        db,
        r2_manager.http.clone(),
        Method::PUT,
        "/lifecycle",
        Some(policy),
    )
    .await
}

#[tauri::command]
#[instrument(skip(db, r2_manager))]
pub async fn r2_delete_lifecycle(
    connection_id: String,
    bucket_name: String,
    jurisdiction: Option<String>,
    db: State<'_, Arc<HostDb>>,
    r2_manager: State<'_, Arc<R2Manager>>,
) -> Result<Value, R2Error> {
    bucket_json_command(
        connection_id,
        bucket_name,
        jurisdiction,
        db,
        r2_manager.http.clone(),
        Method::DELETE,
        "/lifecycle",
        None,
    )
    .await
}

#[tauri::command]
#[instrument(skip(db, r2_manager))]
pub async fn r2_get_managed_domain(
    connection_id: String,
    bucket_name: String,
    jurisdiction: Option<String>,
    db: State<'_, Arc<HostDb>>,
    r2_manager: State<'_, Arc<R2Manager>>,
) -> Result<Value, R2Error> {
    bucket_json_command(
        connection_id,
        bucket_name,
        jurisdiction,
        db,
        r2_manager.http.clone(),
        Method::GET,
        "/domains/managed",
        None,
    )
    .await
}

#[tauri::command]
#[instrument(skip(db, r2_manager))]
pub async fn r2_update_managed_domain(
    connection_id: String,
    bucket_name: String,
    jurisdiction: Option<String>,
    enabled: bool,
    db: State<'_, Arc<HostDb>>,
    r2_manager: State<'_, Arc<R2Manager>>,
) -> Result<Value, R2Error> {
    bucket_json_command(
        connection_id,
        bucket_name,
        jurisdiction,
        db,
        r2_manager.http.clone(),
        Method::PUT,
        "/domains/managed",
        Some(json!({ "enabled": enabled })),
    )
    .await
}

#[tauri::command]
#[instrument(skip(db, r2_manager))]
pub async fn r2_list_custom_domains(
    connection_id: String,
    bucket_name: String,
    jurisdiction: Option<String>,
    db: State<'_, Arc<HostDb>>,
    r2_manager: State<'_, Arc<R2Manager>>,
) -> Result<Value, R2Error> {
    bucket_json_command(
        connection_id,
        bucket_name,
        jurisdiction,
        db,
        r2_manager.http.clone(),
        Method::GET,
        "/domains/custom",
        None,
    )
    .await
}

#[tauri::command]
#[instrument(skip(db, r2_manager))]
pub async fn r2_attach_custom_domain(
    connection_id: String,
    bucket_name: String,
    jurisdiction: Option<String>,
    request: R2AttachCustomDomainRequest,
    db: State<'_, Arc<HostDb>>,
    r2_manager: State<'_, Arc<R2Manager>>,
) -> Result<Value, R2Error> {
    validate_custom_domain(&request.domain)?;
    validate_cloudflare_id(&request.zone_id, "Cloudflare zone ID")?;
    let mut body = json!({
        "domain": request.domain,
        "zoneId": request.zone_id,
    });
    if let Some(enabled) = request.enabled {
        body["enabled"] = json!(enabled);
    }
    if let Some(min_tls) = request.min_tls.filter(|value| !value.trim().is_empty()) {
        body["minTLS"] = json!(min_tls);
    }
    bucket_json_command(
        connection_id,
        bucket_name,
        jurisdiction,
        db,
        r2_manager.http.clone(),
        Method::POST,
        "/domains/custom",
        Some(body),
    )
    .await
}

#[tauri::command]
#[instrument(skip(db, r2_manager, settings))]
pub async fn r2_update_custom_domain(
    connection_id: String,
    bucket_name: String,
    domain: String,
    jurisdiction: Option<String>,
    settings: Value,
    db: State<'_, Arc<HostDb>>,
    r2_manager: State<'_, Arc<R2Manager>>,
) -> Result<Value, R2Error> {
    validate_custom_domain(&domain)?;
    validate_non_empty_json_object(&settings, "custom domain settings")?;
    let encoded_domain = percent_encode_path_segment(&domain);
    bucket_json_command(
        connection_id,
        bucket_name,
        jurisdiction,
        db,
        r2_manager.http.clone(),
        Method::PUT,
        &format!("/domains/custom/{encoded_domain}"),
        Some(settings),
    )
    .await
}

#[tauri::command]
#[instrument(skip(db, r2_manager))]
pub async fn r2_delete_custom_domain(
    connection_id: String,
    bucket_name: String,
    domain: String,
    jurisdiction: Option<String>,
    db: State<'_, Arc<HostDb>>,
    r2_manager: State<'_, Arc<R2Manager>>,
) -> Result<Value, R2Error> {
    validate_custom_domain(&domain)?;
    let encoded_domain = percent_encode_path_segment(&domain);
    bucket_json_command(
        connection_id,
        bucket_name,
        jurisdiction,
        db,
        r2_manager.http.clone(),
        Method::DELETE,
        &format!("/domains/custom/{encoded_domain}"),
        None,
    )
    .await
}

#[tauri::command]
#[instrument(skip(db, r2_manager))]
pub async fn r2_get_metrics(
    connection_id: String,
    db: State<'_, Arc<HostDb>>,
    r2_manager: State<'_, Arc<R2Manager>>,
) -> Result<Value, R2Error> {
    let client = client_for_connection(connection_id, db, r2_manager.http.clone()).await?;
    client
        .request_value(Method::GET, "/r2/metrics", None, None)
        .await
}

async fn bucket_json_command(
    connection_id: String,
    bucket_name: String,
    jurisdiction: Option<String>,
    db: State<'_, Arc<HostDb>>,
    http: Client,
    method: Method,
    suffix: &str,
    body: Option<Value>,
) -> Result<Value, R2Error> {
    validate_path_segment(&bucket_name, "bucket name")?;
    let client = client_for_connection(connection_id, db, http).await?;
    client
        .request_value(
            method,
            &format!("/r2/buckets/{bucket_name}{suffix}"),
            jurisdiction.as_deref(),
            body,
        )
        .await
}

async fn client_for_connection(
    connection_id: String,
    db: State<'_, Arc<HostDb>>,
    http: Client,
) -> Result<R2Client, R2Error> {
    let auth = load_r2_auth(connection_id, db).await?;
    Ok(R2Client {
        http,
        account_id: auth.account_id,
        api_token: auth.api_token,
    })
}

async fn load_r2_auth(
    connection_id: String,
    db: State<'_, Arc<HostDb>>,
) -> Result<R2Auth, R2Error> {
    let db = Arc::clone(&db);
    let id = connection_id.clone();
    let connections = tokio::task::spawn_blocking(move || db.list_s3_connections())
        .await
        .map_err(|e| R2Error::Io(format!("task panicked: {e}")))?
        .map_err(|e| R2Error::Io(e.to_string()))?;

    let connection = connections
        .into_iter()
        .find(|connection| connection.id == id)
        .ok_or_else(|| R2Error::ConnectionNotFound(connection_id.clone()))?;

    if connection.provider != "r2" {
        return Err(R2Error::NotR2Connection);
    }

    let account_id = connection
        .r2_account_id
        .filter(|value| !value.trim().is_empty())
        .ok_or(R2Error::MissingAccountId)?;
    validate_cloudflare_id(&account_id, "Cloudflare account ID")?;

    let vault_key = crate::s3::commands::r2_admin_vault_key(&connection_id);
    let credential = tokio::task::spawn_blocking(move || crate::vault::get_credential(&vault_key))
        .await
        .map_err(|e| R2Error::Io(format!("task panicked: {e}")))?
        .map_err(|_| R2Error::MissingApiToken)?;

    let api_token = match credential {
        crate::vault::StoredCredential::Password { password } if !password.trim().is_empty() => {
            password
        }
        _ => return Err(R2Error::MissingApiToken),
    };

    Ok(R2Auth {
        account_id,
        api_token,
    })
}

fn validate_path_segment(value: &str, label: &str) -> Result<(), R2Error> {
    if value.trim().is_empty() {
        return Err(R2Error::InvalidRequest(format!("{label} is required")));
    }
    if value.contains('/') || value.contains('\\') || value.contains('?') || value.contains('#') {
        return Err(R2Error::InvalidRequest(format!(
            "{label} cannot contain path separators or query fragments"
        )));
    }
    Ok(())
}

fn validate_cloudflare_id(value: &str, label: &str) -> Result<(), R2Error> {
    if value.len() != 32 || !value.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(R2Error::InvalidRequest(format!(
            "{label} must be a 32-character hex string"
        )));
    }
    Ok(())
}

fn validate_custom_domain(domain: &str) -> Result<(), R2Error> {
    if domain.is_empty() || domain.len() > 253 {
        return Err(R2Error::InvalidRequest(
            "domain must be between 1 and 253 characters".to_string(),
        ));
    }

    for label in domain.split('.') {
        if label.is_empty() || label.len() > 63 {
            return Err(R2Error::InvalidRequest(
                "domain labels must be between 1 and 63 characters".to_string(),
            ));
        }
        if label.starts_with('-') || label.ends_with('-') {
            return Err(R2Error::InvalidRequest(
                "domain labels cannot start or end with '-'".to_string(),
            ));
        }
        if !label
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
        {
            return Err(R2Error::InvalidRequest(
                "domain can only contain ASCII letters, numbers, hyphens, and dots".to_string(),
            ));
        }
    }

    Ok(())
}

fn percent_encode_path_segment(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(byte as char);
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

fn validate_json_object(value: &Value, label: &str) -> Result<(), R2Error> {
    if !value.is_object() {
        return Err(R2Error::InvalidRequest(format!(
            "{label} must be a JSON object"
        )));
    }
    let bytes = serde_json::to_vec(value)
        .map_err(|e| R2Error::InvalidRequest(format!("could not serialize {label}: {e}")))?;
    if bytes.len() > MAX_JSON_BODY_BYTES {
        return Err(R2Error::InvalidRequest(format!(
            "{label} must be {MAX_JSON_BODY_BYTES} bytes or smaller"
        )));
    }
    Ok(())
}

fn validate_non_empty_json_object(value: &Value, label: &str) -> Result<(), R2Error> {
    validate_json_object(value, label)?;
    if value.as_object().is_some_and(|object| object.is_empty()) {
        return Err(R2Error::InvalidRequest(format!(
            "{label} must include at least one setting"
        )));
    }
    Ok(())
}

fn decode_error(status: StatusCode, body: &str, error: &serde_json::Error) -> R2Error {
    R2Error::Decode(format!(
        "HTTP {status}: invalid response: {error}; body: {}",
        body_snippet(body)
    ))
}

fn body_snippet(body: &str) -> String {
    let snippet: String = body.chars().take(240).collect();
    if body.chars().count() > 240 {
        format!("{snippet}...")
    } else {
        snippet
    }
}

fn cloudflare_error<T>(envelope: &CloudflareEnvelope<T>) -> R2Error {
    R2Error::Api {
        code: first_error_code(envelope),
        message: cloudflare_message(envelope),
    }
}

fn first_error_code<T>(envelope: &CloudflareEnvelope<T>) -> Option<i64> {
    envelope
        .errors
        .iter()
        .chain(envelope.messages.iter())
        .find_map(|value| value.as_object()?.get("code")?.as_i64())
}

fn cloudflare_message<T>(envelope: &CloudflareEnvelope<T>) -> String {
    let mut parts: Vec<String> = envelope
        .errors
        .iter()
        .chain(envelope.messages.iter())
        .filter_map(message_from_value)
        .collect();
    if parts.is_empty() {
        parts.push("request failed".to_string());
    }
    parts.join("; ")
}

fn message_from_value(value: &Value) -> Option<String> {
    match value {
        Value::String(message) if !message.is_empty() => Some(message.clone()),
        Value::Object(map) => map
            .get("message")
            .and_then(Value::as_str)
            .filter(|message| !message.is_empty())
            .map(|message| match map.get("code").and_then(Value::as_i64) {
                Some(code) => format!("{code}: {message}"),
                None => message.to_string(),
            }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloudflare_message_includes_error_code_and_text() {
        let envelope: CloudflareEnvelope<Value> = CloudflareEnvelope {
            success: false,
            errors: vec![json!({ "code": 10013, "message": "Invalid API token" })],
            messages: Vec::new(),
            result: None,
        };

        assert_eq!(cloudflare_message(&envelope), "10013: Invalid API token");
    }

    #[test]
    fn api_error_serializes_cloudflare_code() {
        let value = serde_json::to_value(R2Error::Api {
            code: Some(10013),
            message: "Invalid API token".to_string(),
        })
        .expect("serialize R2Error");

        assert_eq!(value["kind"], "cloudflare_api");
        assert_eq!(value["code"], 10013);
    }

    #[test]
    fn rejects_unsafe_bucket_segments() {
        let err = validate_path_segment("bucket/name", "bucket name").expect_err("slash is unsafe");
        assert!(matches!(err, R2Error::InvalidRequest(_)));
    }

    #[test]
    fn accepts_frontend_min_tls_contract() {
        let request: R2AttachCustomDomainRequest = serde_json::from_value(json!({
            "domain": "files.example.com",
            "zoneId": "0123456789abcdef0123456789abcdef",
            "minTls": "1.2",
        }))
        .expect("deserialize request");

        assert_eq!(request.min_tls.as_deref(), Some("1.2"));
    }

    #[test]
    fn rejects_unsafe_custom_domain() {
        let err = validate_custom_domain("files.example.com:443").expect_err("colon is unsafe");
        assert!(matches!(err, R2Error::InvalidRequest(_)));
    }

    #[test]
    fn rejects_non_object_json_policy() {
        let err = validate_json_object(&json!([]), "CORS policy").expect_err("array root rejected");
        assert!(matches!(err, R2Error::InvalidRequest(_)));
    }

    #[test]
    fn rejects_empty_custom_domain_settings() {
        let err = validate_non_empty_json_object(&json!({}), "custom domain settings")
            .expect_err("empty update is not useful");
        assert!(matches!(err, R2Error::InvalidRequest(_)));
    }

    #[test]
    fn r2_auth_debug_redacts_token() {
        let debug = format!(
            "{:?}",
            R2Auth {
                account_id: "0123456789abcdef0123456789abcdef".to_string(),
                api_token: "secret-token".to_string(),
            }
        );

        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("secret-token"));
    }
}
