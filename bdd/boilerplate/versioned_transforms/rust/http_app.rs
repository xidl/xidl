//! The directory application server over real HTTP: version negotiation,
//! storage and the caller-aware write rules, driven through the generated
//! Rust dispatchers, exactly like the TypeScript application server.

use crate::gen::version_adapters::{
    dispatch_directory_profile, dispatch_directory_publish, AdaptationError, DirectoryProfileOutcome,
    DirectoryProfileRequestV20260701, DirectoryPublishOutcome, DirectoryPublishRequestV20260701,
    HeadOutcome, ProjectionOutcome, UnsupportedProjection, Version, VersionContext,
};
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
    collections::{BTreeMap, HashMap},
    sync::{
        atomic::{AtomicUsize, Ordering},
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
    pub writes: AtomicUsize,
}

fn is_calendar_date(value: &str) -> bool {
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

fn resolve_version(header: Option<&str>) -> Option<Version> {
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

fn header<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

fn echo(mut response: Response, version: Option<Version>) -> Response {
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

fn reason(status: StatusCode, body: &str, version: Option<Version>) -> Response {
    echo(
        (status, Json(json!({ "reason": body }))).into_response(),
        version,
    )
}

fn storage_for(app: &AppState) -> Storage {
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
fn remember_facts(app: &AppState, id: &str, raw: &Value) {
    let mut rows = app.rows.lock().unwrap();
    let previous = rows.get(id).cloned().unwrap_or_default();
    let created_at = match raw.get("created_at") {
        Some(Value::Number(number)) => number.as_i64().map(|value| value as i32),
        _ => match raw.get("created_at_utc").and_then(Value::as_str) {
            Some(rfc3339) => crate::hooks::rfc3339_to_epoch(rfc3339),
            None => None,
        },
    };
    let note = raw
        .get("note")
        .and_then(|note| note.get("text"))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| {
            raw.get("notes")
                .and_then(Value::as_array)
                .and_then(|list| list.first())
                .and_then(|note| note.get("text"))
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .or(previous.note);
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

async fn publish(
    State(app): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let raw_header = header(&headers, "x-transform-version");
    let Some(version) = resolve_version(raw_header) else {
        return echo(
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "reason": "unsupported X-Transform-Version" })),
            )
                .into_response(),
            None,
        );
    };
    let raw: Value = match serde_json::from_slice(&body) {
        Ok(raw) => raw,
        Err(_) => return reason(StatusCode::BAD_REQUEST, "malformed JSON body", Some(version)),
    };
    let hooks = crate::hooks::Hooks::default();
    let state = app.clone();
    let facts = raw.clone();
    let result = dispatch_directory_publish(
        version,
        json!({ "profile": raw }),
        move |input: DirectoryPublishRequestV20260701, context: VersionContext| {
            let state = state.clone();
            let facts = facts.clone();
            async move {
                let mut profile = input.profile;
                let previous = state.ledger.lock().unwrap().get(&profile.id).cloned();
                let facts_before = state.rows.lock().unwrap().get(&profile.id).cloned();
                let old_caller = context.original_version != Version::V20260701;
                // E3 write rule: a caller restating the stored name pair keeps
                // the custom display; a caller changing the names gets the
                // mechanical merge the hook computed.
                if old_caller {
                    if let Some(facts_before) = facts_before {
                        let same_names = facts.get("first_name").and_then(Value::as_str)
                            == facts_before.first_name.as_deref()
                            && facts.get("last_name").and_then(Value::as_str)
                                == facts_before.last_name.as_deref();
                        if same_names {
                            if let Some(previous) = &previous {
                                profile.display_name = previous.display_name.clone();
                            }
                        }
                    }
                }
                // E7 write rule: only older callers write through the narrow
                // view; a stated note replaces the visible head and keeps the
                // entries it cannot see, an omitted note changes nothing.
                profile.notes = if !old_caller {
                    profile.notes
                } else if profile.notes.is_empty() {
                    previous.iter().flat_map(|p| p.notes.clone()).collect()
                } else {
                    let mut notes = vec![profile.notes[0].clone()];
                    if let Some(previous) = &previous {
                        notes.extend(previous.notes.iter().skip(1).cloned());
                    }
                    notes
                };
                remember_facts(&state, &profile.id, &facts);
                state.writes.fetch_add(1, Ordering::SeqCst);
                state.ledger.lock().unwrap().insert(profile.id.clone(), profile.clone());
                // The stored snapshot is taken after the commit: the response
                // projection reads exactly what this command landed.
                let stored = storage_for(&state);
                HeadOutcome {
                    outcome: Ok(profile),
                    stored,
                }
            }
        },
        &hooks,
    )
    .await;
    match result {
        Err(AdaptationError::Validation(error))
        | Err(AdaptationError::Unsupported(UnsupportedProjection { reason: error, .. })) => {
            reason(StatusCode::BAD_REQUEST, &error, Some(version))
        }
        Ok(ProjectionOutcome::Unsupported { error, .. }) => {
            reason(StatusCode::UNPROCESSABLE_ENTITY, &error.reason, Some(version))
        }
        Ok(ProjectionOutcome::Projected(outcome)) => published_response(outcome, version),
    }
}

fn published_response(outcome: DirectoryPublishOutcome, version: Version) -> Response {
    let (value, error) = match outcome {
        DirectoryPublishOutcome::V20260101(result) => match result {
            Ok(value) => (Some(json!(value)), None),
            Err(error) => (None, Some(error)),
        },
        DirectoryPublishOutcome::V20260401(result) => match result {
            Ok(value) => (Some(json!(value)), None),
            Err(error) => (None, Some(error)),
        },
        DirectoryPublishOutcome::V20260701(result) => match result {
            Ok(value) => (Some(json!(value)), None),
            Err(error) => (None, Some(error)),
        },
    };
    match (value, error) {
        (Some(value), _) => echo((StatusCode::OK, Json(value)).into_response(), Some(version)),
        (None, Some(error)) => {
            let status = StatusCode::from_u16(error.code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            reason(status, &error.message, Some(version))
        }
        (None, None) => reason(StatusCode::INTERNAL_SERVER_ERROR, "unreachable", Some(version)),
    }
}

async fn profile(
    State(app): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let raw_header = header(&headers, "x-transform-version");
    let echoed = resolve_version(raw_header);
    let Some(version) = echoed else {
        return echo(
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "reason": "unsupported X-Transform-Version" })),
            )
                .into_response(),
            None,
        );
    };
    let id = params.get("id").cloned().unwrap_or_default();
    let hooks = crate::hooks::Hooks::default();
    let stored = storage_for(&app);
    let state = app.clone();
    let result = dispatch_directory_profile(
        version,
        json!({ "id": id }),
        move |request: DirectoryProfileRequestV20260701, _context: VersionContext| {
            let state = state.clone();
            let stored = stored.clone();
            async move {
                let head = state.ledger.lock().unwrap().get(&request.id).cloned();
                HeadOutcome {
                    outcome: match head {
                        Some(profile) => Ok(profile),
                        None => Err(v2026_07_01::DirectoryProfileError::Missing(
                            v2026_07_01::Missing { id: request.id },
                        )),
                    },
                    stored,
                }
            }
        },
        &hooks,
    )
    .await;
    match result {
        Err(AdaptationError::Validation(error))
        | Err(AdaptationError::Unsupported(UnsupportedProjection { reason: error, .. })) => {
            reason(StatusCode::BAD_REQUEST, &error, echoed)
        }
        Ok(ProjectionOutcome::Unsupported { error, .. }) => {
            reason(StatusCode::UNPROCESSABLE_ENTITY, &error.reason, echoed)
        }
        Ok(ProjectionOutcome::Projected(DirectoryProfileOutcome::V20260101(Ok(value)))) => {
            echo((StatusCode::OK, Json(json!(value))).into_response(), echoed)
        }
        Ok(ProjectionOutcome::Projected(DirectoryProfileOutcome::V20260401(Ok(value)))) => {
            echo((StatusCode::OK, Json(json!(value))).into_response(), echoed)
        }
        Ok(ProjectionOutcome::Projected(DirectoryProfileOutcome::V20260701(Ok(value)))) => {
            echo((StatusCode::OK, Json(json!(value))).into_response(), echoed)
        }
        Ok(ProjectionOutcome::Projected(DirectoryProfileOutcome::V20260101(Err(_))))
        | Ok(ProjectionOutcome::Projected(DirectoryProfileOutcome::V20260401(Err(_))))
        | Ok(ProjectionOutcome::Projected(DirectoryProfileOutcome::V20260701(Err(_)))) => {
            reason(StatusCode::NOT_FOUND, "no such profile", echoed)
        }
    }
}

/// The application router: real HTTP over the generated dispatchers.
pub fn directory_router(app: Arc<AppState>) -> Router {
    Router::new()
        .route("/profiles", post(publish).get(profile))
        .with_state(app)
}
