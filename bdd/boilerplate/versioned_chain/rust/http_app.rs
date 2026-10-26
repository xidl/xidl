//! The booking application server over real HTTP: version negotiation,
//! storage, idempotency and behaviour switches, driven through the generated
//! Rust dispatchers, exactly like the TypeScript application server.

use crate::gen::version_adapters::{
    dispatch_scheduler_book, dispatch_scheduler_get, AdaptationError, HeadOutcome,
    ProjectionOutcome, SchedulerBookOutcome, SchedulerBookRequestV20270101, SchedulerGetOutcome,
    SchedulerGetRequestV20270101, UnsupportedProjection, Version, VersionContext,
};
use crate::gen::v2027_01_01;
use crate::hooks::{Hooks, Storage};
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
    net::SocketAddr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

/// Storage only moves forward: one row per booking, facts merged in on commit.
#[derive(Default)]
pub struct Rows {
    pub agenda: BTreeMap<String, String>,
    pub rosters: BTreeMap<String, Vec<String>>,
}

#[derive(Default)]
pub struct AppState {
    pub rows: Mutex<Rows>,
    /// Fingerprints live on the upcast command: same meaning, same fingerprint.
    pub ledger: Mutex<BTreeMap<String, (String, v2027_01_01::Booking)>>,
    pub writes: AtomicUsize,
}

fn is_calendar_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    let (Ok(year), Ok(month), Ok(day)) = (
        value[0..4].parse::<i64>(),
        value[5..7].parse::<u32>(),
        value[8..10].parse::<u32>(),
    ) else {
        return false;
    };
    if month == 0 || month > 12 || day == 0 {
        return false;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let february = if leap { 29 } else { 28 };
    let days = [31, february, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    day <= days[(month - 1) as usize]
}

/// This application's example policy: absent header serves Head; a date resolves
/// to the newest served release not later than it; anything else is unresolved.
fn resolve_version(header: Option<&str>) -> Option<Version> {
    let Some(header) = header else {
        return Some(Version::V20270101);
    };
    if !is_calendar_date(header) {
        return None;
    }
    if header >= "2027-01-01" {
        Some(Version::V20270101)
    } else if header >= "2026-10-01" {
        Some(Version::V20261001)
    } else if header >= "2026-06-01" {
        Some(Version::V20260601)
    } else {
        None
    }
}

/// Runtime base four: named switches with a start date, consulted on the
/// resolved version.
fn reject_blank_title_on(served_date: &str) -> bool {
    served_date >= "2027-01-01"
}

fn header<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

fn echo(mut response: Response, version: Option<Version>) -> Response {
    let label = version
        .map(|version| match version {
            Version::V20260601 => "2026-06-01",
            Version::V20261001 => "2026-10-01",
            Version::V20270101 => "2027-01-01",
        })
        .unwrap_or("unresolved");
    response.headers_mut().insert(
        "x-booking-version",
        HeaderValue::from_str(label).expect("a version label is a valid header value"),
    );
    response
}

fn reason(status: StatusCode, reason: &str, version: Option<Version>) -> Response {
    echo((status, Json(json!({ "reason": reason }))).into_response(), version)
}

fn fingerprint(booking: &v2027_01_01::Booking) -> String {
    let items: Vec<String> = booking
        .items
        .iter()
        .map(|entry| format!("{};{}", entry.title, entry.minutes))
        .collect();
    format!("book|{}|{}|{}", booking.id, booking.title, items.join(","))
}

async fn book(
    State(app): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let raw_header = header(&headers, "x-booking-version");
    let Some(version) = resolve_version(raw_header) else {
        return echo(
            (StatusCode::BAD_REQUEST, Json(json!({ "reason": "unsupported X-Booking-Version" })))
                .into_response(),
            None,
        );
    };
    let Some(key) = header(&headers, "idempotency-key").map(str::to_owned) else {
        return reason(
            StatusCode::BAD_REQUEST,
            "idempotency-key header is required",
            Some(version),
        );
    };
    let raw: Value = match serde_json::from_slice(&body) {
        Ok(raw) => raw,
        Err(_) => {
            return reason(StatusCode::BAD_REQUEST, "malformed JSON body", Some(version));
        }
    };
    let hooks = Hooks::default();
    let stored_for_request = {
        let rows = app.rows.lock().unwrap();
        Storage {
            agenda_texts: rows.agenda.clone(),
            rosters: rows.rosters.clone(),
        }
    };
    let state = app.clone();
    let facts = raw.clone();
    let result = dispatch_scheduler_book(
        version,
        json!({ "booking": raw }),
        move |input: SchedulerBookRequestV20270101, _context: VersionContext| {
            let state = state.clone();
            let key = key.clone();
            let stored_for_request = stored_for_request.clone();
            let facts = facts.clone();
            async move {
                let fingerprint = fingerprint(&input.booking);
                let cached = state.ledger.lock().unwrap().get(&key).cloned();
                if let Some((cached_fingerprint, cached_outcome)) = cached {
                    // A replay answers before any behaviour gate.
                    if cached_fingerprint == fingerprint {
                        return HeadOutcome {
                            outcome: Ok(cached_outcome),
                            stored: stored_for_request,
                        };
                    }
                    return HeadOutcome {
                        outcome: Err(xidl_rust_axum::Error::new(
                            409,
                            "idempotency key conflict",
                        )),
                        stored: stored_for_request,
                    };
                }
                if reject_blank_title_on("2027-01-01") && input.booking.title.trim().is_empty() {
                    return HeadOutcome {
                        outcome: Err(xidl_rust_axum::Error::new(422, "blank title")),
                        stored: stored_for_request,
                    };
                }
                state.writes.fetch_add(1, Ordering::SeqCst);
                {
                    // Facts land when the command commits; refused requests
                    // change nothing at all.
                    let id = match facts.get("id") {
                        Some(Value::String(id)) => id.clone(),
                        Some(Value::Number(number)) => number.to_string(),
                        _ => String::new(),
                    };
                    let mut rows = state.rows.lock().unwrap();
                    let agenda = facts.get("agenda").and_then(Value::as_str);
                    let roster = facts.get("roster").and_then(Value::as_array).map(|list| {
                        list.iter().filter_map(Value::as_str).map(str::to_owned).collect::<Vec<_>>()
                    });
                    if let Some(agenda) = agenda {
                        rows.agenda.insert(id.clone(), agenda.to_owned());
                    }
                    if let Some(roster) = roster {
                        rows.rosters.insert(id.clone(), roster);
                    }
                }
                let mut ledger = state.ledger.lock().unwrap();
                ledger.insert(key, (fingerprint.clone(), input.booking.clone()));
                ledger.insert(
                    format!("head:{}", input.booking.id),
                    (fingerprint, input.booking.clone()),
                );
                HeadOutcome {
                    outcome: Ok(input.booking),
                    stored: stored_for_request,
                }
            }
        },
        &hooks,
    )
    .await;
    match result {
        Err(AdaptationError::Validation(error)) | Err(AdaptationError::Unsupported(UnsupportedProjection { reason: error, .. })) => {
            reason(StatusCode::BAD_REQUEST, &error, Some(version))
        }
        Ok(ProjectionOutcome::Unsupported { error, .. }) => {
            reason(StatusCode::UNPROCESSABLE_ENTITY, &error.reason, Some(version))
        }
        Ok(ProjectionOutcome::Projected(SchedulerBookOutcome::V20260601(Ok(value)))) => {
            echo((StatusCode::OK, Json(json!(value))).into_response(), Some(version))
        }
        Ok(ProjectionOutcome::Projected(SchedulerBookOutcome::V20261001(Ok(value)))) => {
            echo((StatusCode::OK, Json(json!(value))).into_response(), Some(version))
        }
        Ok(ProjectionOutcome::Projected(SchedulerBookOutcome::V20270101(Ok(value)))) => {
            echo((StatusCode::OK, Json(json!(value))).into_response(), Some(version))
        }
        Ok(ProjectionOutcome::Projected(SchedulerBookOutcome::V20260601(Err(error))))
        | Ok(ProjectionOutcome::Projected(SchedulerBookOutcome::V20261001(Err(error))))
        | Ok(ProjectionOutcome::Projected(SchedulerBookOutcome::V20270101(Err(error)))) => {
            let status = StatusCode::from_u16(error.code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            reason(status, &error.message, Some(version))
        }
    }
}

async fn get_booking(
    State(app): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let raw_header = header(&headers, "x-booking-version");
    let version = resolve_version(raw_header);
    let echoed = version;
    let Some(version) = version else {
        return echo(
            (StatusCode::BAD_REQUEST, Json(json!({ "reason": "unsupported X-Booking-Version" })))
                .into_response(),
            None,
        );
    };
    let id = params.get("id").cloned().unwrap_or_default();
    let hooks = Hooks::default();
    let stored_for_request = {
        let rows = app.rows.lock().unwrap();
        Storage {
            agenda_texts: rows.agenda.clone(),
            rosters: rows.rosters.clone(),
        }
    };
    let state = app.clone();
    let result = dispatch_scheduler_get(
        version,
        json!({ "id": id }),
        move |request: SchedulerGetRequestV20270101, _context: VersionContext| {
            let state = state.clone();
            let stored_for_request = stored_for_request.clone();
            async move {
                let head = state
                    .ledger
                    .lock()
                    .unwrap()
                    .get(&format!("head:{}", request.id))
                    .cloned();
                HeadOutcome {
                    outcome: match head {
                        Some((_, booking)) => Ok(booking),
                        None => Err(v2027_01_01::SchedulerGetError::NotFound(v2027_01_01::NotFound {
                            id: request.id,
                        })),
                    },
                    stored: stored_for_request,
                }
            }
        },
        &hooks,
    )
    .await;
    match result {
        Err(AdaptationError::Validation(error)) | Err(AdaptationError::Unsupported(UnsupportedProjection { reason: error, .. })) => {
            reason(StatusCode::BAD_REQUEST, &error, echoed)
        }
        Ok(ProjectionOutcome::Unsupported { error, .. }) => {
            reason(StatusCode::UNPROCESSABLE_ENTITY, &error.reason, echoed)
        }
        Ok(ProjectionOutcome::Projected(SchedulerGetOutcome::V20260601(Ok(value)))) => {
            echo((StatusCode::OK, Json(json!(value))).into_response(), echoed)
        }
        Ok(ProjectionOutcome::Projected(SchedulerGetOutcome::V20261001(Ok(value)))) => {
            echo((StatusCode::OK, Json(json!(value))).into_response(), echoed)
        }
        Ok(ProjectionOutcome::Projected(SchedulerGetOutcome::V20270101(Ok(value)))) => {
            echo((StatusCode::OK, Json(json!(value))).into_response(), echoed)
        }
        Ok(ProjectionOutcome::Projected(SchedulerGetOutcome::V20260601(Err(_))))
        | Ok(ProjectionOutcome::Projected(SchedulerGetOutcome::V20261001(Err(_)))) => {
            reason(StatusCode::NOT_FOUND, "no such booking", echoed)
        }
        Ok(ProjectionOutcome::Projected(SchedulerGetOutcome::V20270101(Err(_)))) => {
            reason(StatusCode::NOT_FOUND, "no such booking", echoed)
        }
    }
}

/// The application router: real HTTP over the generated dispatchers.
pub fn booking_router(app: Arc<AppState>) -> Router {
    Router::new()
        .route("/bookings", post(book).get(get_booking))
        .with_state(app)
}
