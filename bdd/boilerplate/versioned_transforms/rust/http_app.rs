//! The directory application server over real HTTP: version negotiation,
//! storage and the caller-aware write rules, driven through the generated
//! Rust dispatchers, exactly like the TypeScript application server.

use crate::gen::version_adapters::{
    dispatch_directory_profile, dispatch_directory_publish, AdaptationError, DirectoryProfileOutcome,
    DirectoryProfileRequestV20260701, DirectoryPublishOutcome, DirectoryPublishRequestV20260701,
    HeadOutcome, ProjectionOutcome, UnsupportedProjection, Version, VersionContext,
};
use crate::routes;

use crate::gen::v2026_01_01;
use crate::gen::v2026_04_01;
use crate::gen::v2026_07_01;
use crate::hooks::Storage;
use axum::{
    body::Bytes,
    extract::{Query, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::{
        atomic::AtomicUsize,
        Arc, Mutex,
    },
};

/// What older releases still read, keyed by profile id.
#[derive(Clone, Default)]
pub struct Facts {
    pub created_at: Option<i32>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub note: Option<String>,
    pub status: Option<String>,
    pub recipient: Option<String>,
}

#[derive(Default)]
pub struct AppState {
    pub rows: Mutex<BTreeMap<String, Facts>>,
    pub ledger: Mutex<BTreeMap<String, v2026_07_01::Profile>>,
    /// Profiles the application sealed: Head raises the refinement for them.
    pub sealed: Mutex<BTreeSet<String>>,
    pub writes: AtomicUsize,
}

pub(crate) fn is_calendar_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    let (Ok(_year), Ok(month), Ok(day)) = (
        value[0..4].parse::<i64>(),
        value[5..7].parse::<u32>(),
        value[8..10].parse::<u32>(),
    ) else {
        return false;
    };
    let leap_years = |year: i64| year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let Ok(year) = value[0..4].parse::<i64>() else {
        return false;
    };
    if month == 0 || month > 12 || day == 0 {
        return false;
    }
    let february = if leap_years(year) { 29 } else { 28 };
    let days = [31, february, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    day <= days[(month - 1) as usize]
}

pub(crate) fn resolve_version(header: Option<&str>) -> Option<Version> {
    let Some(header) = header else {
        return Some(Version::V20260701);
    };
    if !is_calendar_date(header) {
        return None;
    }
    if header >= "2026-07-01" {
        Some(Version::V20260701)
    } else if header >= "2026-04-01" {
        Some(Version::V20260401)
    } else if header >= "2026-01-01" {
        Some(Version::V20260101)
    } else {
        None
    }
}

pub(crate) fn header<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

/// The journey's typed client: every leg goes through one request builder so
/// the three endpoints share one version-header rule.
pub(crate) struct JourneyClient {
    base: String,
    http: xidl_rust_axum::reqwest::Client,
}

impl JourneyClient {
    pub(crate) fn new(base: String) -> Self {
        JourneyClient {
            base,
            http: xidl_rust_axum::reqwest::Client::new(),
        }
    }

    async fn send(
        &self,
        method: xidl_rust_axum::reqwest::Method,
        path: &str,
        version: Option<&str>,
        body: Option<Value>,
    ) -> xidl_rust_axum::reqwest::Response {
        let mut request = self
            .http
            .request(method, format!("{}{path}", self.base));
        if let Some(version) = version {
            request = request.header("x-transform-version", version);
        }
        if let Some(body) = body {
            request = request.json(&body);
        }
        request.send().await.unwrap()
    }

    pub(crate) async fn get(
        &self,
        version: Option<&str>,
        id: &str,
    ) -> xidl_rust_axum::reqwest::Response {
        self.send(
            xidl_rust_axum::reqwest::Method::GET,
            &format!("/profiles?id={id}"),
            version,
            None,
        )
        .await
    }

    pub(crate) async fn post(
        &self,
        version: Option<&str>,
        body: Value,
    ) -> xidl_rust_axum::reqwest::Response {
        self.send(
            xidl_rust_axum::reqwest::Method::POST,
            "/profiles",
            version,
            Some(body),
        )
        .await
    }

    pub(crate) async fn roster(&self, version: Option<&str>) -> xidl_rust_axum::reqwest::Response {
        self.send(
            xidl_rust_axum::reqwest::Method::GET,
            "/roster",
            version,
            None,
        )
        .await
    }
}

pub(crate) fn echo(mut response: Response, version: Option<Version>) -> Response {
    let label = match version {
        Some(Version::V20260101) => "2026-01-01",
        Some(Version::V20260401) => "2026-04-01",
        Some(Version::V20260701) => "2026-07-01",
        None => "unresolved",
    };
    response.headers_mut().insert(
        "x-transform-version",
        HeaderValue::from_str(label).expect("a version label is a valid header value"),
    );
    response
}

pub(crate) fn reason(status: StatusCode, body: &str, version: Option<Version>) -> Response {
    echo(
        (status, Json(json!({ "reason": body }))).into_response(),
        version,
    )
}

pub(crate) fn storage_for(app: &AppState) -> Storage {
    let rows = app.rows.lock().unwrap();
    let mut storage = Storage::default();
    for (id, facts) in rows.iter() {
        if let Some(created_at) = facts.created_at {
            storage.created_at.insert(id.clone(), created_at);
        }
        if let Some(first_name) = facts.first_name.clone() {
            storage.first_names.insert(id.clone(), first_name);
        }
        if let Some(last_name) = facts.last_name.clone() {
            storage.last_names.insert(id.clone(), last_name);
        }
        if let Some(note) = facts.note.clone() {
            storage.notes.insert(id.clone(), v2026_04_01::Note { text: note });
        }
        if let Some(status) = facts.status.clone() {
            storage.statuses.insert(id.clone(), status);
        }
        if let Some(recipient) = facts.recipient.clone() {
            storage.recipients.insert(id.clone(), recipient);
        }
    }
    storage
}

/// The application stores its own wire facts; only committed facts land.
pub(crate) fn remember_facts(app: &AppState, id: &str, raw: &Value) {
    let mut rows = app.rows.lock().unwrap();
    let previous = rows.get(id).cloned().unwrap_or_default();
    let created_at = match raw.get("created_at") {
        Some(Value::Number(number)) => number.as_i64().map(|value| value as i32),
        _ => match raw.get("created_at_utc").and_then(Value::as_str) {
            Some(rfc3339) => crate::hooks::rfc3339_to_epoch(rfc3339),
            None => None,
        },
    };
    // E7 write rule: a caller that owns the whole list clears the note fact
    // by sending an empty list; callers without the list keep the stored fact.
    let note = match raw.get("notes").and_then(Value::as_array) {
        Some(list) => list
            .first()
            .and_then(|note| note.get("text"))
            .and_then(Value::as_str)
            .map(str::to_owned),
        None => raw
            .get("note")
            .and_then(|note| note.get("text"))
            .and_then(Value::as_str)
            .map(str::to_owned)
            .or(previous.note),
    };
    rows.insert(
        id.to_owned(),
        Facts {
            created_at: created_at.or(previous.created_at),
            first_name: raw
                .get("first_name")
                .and_then(Value::as_str)
                .map(str::to_owned)
                .or(previous.first_name),
            last_name: raw
                .get("last_name")
                .and_then(Value::as_str)
                .map(str::to_owned)
                .or(previous.last_name),
            note,
            status: raw
                .get("status")
                .and_then(Value::as_str)
                .map(str::to_owned)
                .or(previous.status),
            recipient: raw
                .get("recipient")
                .and_then(Value::as_str)
                .map(str::to_owned)
                .or_else(|| {
                    raw.get("to")
                        .and_then(|to| to.get("handle"))
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                })
                .or(previous.recipient),
        },
    );
}

/// The application router: real HTTP over the generated dispatchers.
pub fn directory_router(app: Arc<AppState>) -> Router {
    Router::new()
        .route("/profiles", post(routes::publish).get(routes::profile))
        .route("/roster", axum::routing::get(routes::roster))
        .with_state(app)
}
