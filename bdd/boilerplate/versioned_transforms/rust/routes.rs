//! The directory HTTP handlers: publish, profile reads and the roster,
//! each driving the generated dispatchers over the shared application state.

use crate::gen::v2026_01_01;
use crate::gen::v2026_04_01;
use crate::gen::v2026_07_01;
use crate::gen::version_adapters::{
    dispatch_directory_profile, dispatch_directory_publish, AdaptationError,
    DirectoryProfileOutcome, DirectoryProfileRequestV20260701, DirectoryPublishOutcome,
    DirectoryPublishRequestV20260701, HeadOutcome, ProjectionOutcome, UnsupportedProjection,
    Version, VersionContext,
};
use crate::http_app::{
    echo, header, reason, remember_facts, resolve_version, storage_for, AppState,
};
use axum::{
    body::Bytes,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

pub(crate) async fn publish(
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

pub(crate) fn published_response(outcome: DirectoryPublishOutcome, version: Version) -> Response {
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

pub(crate) async fn profile(
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
                if state.sealed.lock().unwrap().contains(&request.id) {
                    return HeadOutcome {
                        outcome: Err(v2026_07_01::DirectoryProfileError::Sealed(
                            v2026_07_01::Sealed { id: request.id },
                        )),
                        stored,
                    };
                }
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
        Ok(ProjectionOutcome::Projected(DirectoryProfileOutcome::V20260701(Err(
            error,
        )))) => {
            let status = match error {
                v2026_07_01::DirectoryProfileError::Missing(_) => StatusCode::NOT_FOUND,
                v2026_07_01::DirectoryProfileError::Sealed(_) => StatusCode::CONFLICT,
                v2026_07_01::DirectoryProfileError::Framework(ref runtime) => StatusCode
                    ::from_u16(runtime.code)
                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            };
            reason(status, "profile read refused", echoed)
        }
        // Declared errors carry their status; downgraded refinements keep the
        // committed status on the runtime channel.
        Ok(ProjectionOutcome::Projected(DirectoryProfileOutcome::V20260401(Err(
            error,
        )))) => {
            let status = match error {
                v2026_04_01::DirectoryProfileError::Missing(_) => StatusCode::NOT_FOUND,
                v2026_04_01::DirectoryProfileError::Framework(ref runtime) => StatusCode
                    ::from_u16(runtime.code)
                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            };
            reason(status, "profile read refused", echoed)
        }
        Ok(ProjectionOutcome::Projected(DirectoryProfileOutcome::V20260101(Err(
            error,
        )))) => {
            let status = match error {
                v2026_01_01::DirectoryProfileError::Missing(_) => StatusCode::NOT_FOUND,
                v2026_01_01::DirectoryProfileError::Framework(ref runtime) => StatusCode
                    ::from_u16(runtime.code)
                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            };
            reason(status, "profile read refused", echoed)
        }
    }
}

/// E9: the whole roster in one read; every element projects with its own facts.
pub(crate) async fn roster(State(app): State<Arc<AppState>>, headers: HeaderMap) -> Response {
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
    let hooks = crate::hooks::Hooks::default();
    let state = app.clone();
    let result = crate::gen::version_adapters::dispatch_directory_roster(
        version,
        json!({}),
        move |_request, _context: VersionContext| {
            let state = state.clone();
            let stored = storage_for(&state);
            async move {
                let all: Vec<v2026_07_01::Profile> =
                    state.ledger.lock().unwrap().values().cloned().collect();
                HeadOutcome {
                    outcome: if all.is_empty() {
                        Err(xidl_rust_axum::Error::new(500, "no profiles yet"))
                    } else {
                        Ok(all)
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
        Ok(ProjectionOutcome::Projected(outcome)) => {
            // The three element types are the wire shapes of their own
            // release; serialize whichever projected.
            let value = match outcome {
                crate::gen::version_adapters::DirectoryRosterOutcome::V20260101(Ok(value)) => {
                    serde_json::to_value(&value).unwrap_or(Value::Array(Vec::new()))
                }
                crate::gen::version_adapters::DirectoryRosterOutcome::V20260401(Ok(value)) => {
                    serde_json::to_value(&value).unwrap_or(Value::Array(Vec::new()))
                }
                crate::gen::version_adapters::DirectoryRosterOutcome::V20260701(Ok(value)) => {
                    serde_json::to_value(&value).unwrap_or(Value::Array(Vec::new()))
                }
                _ => Value::Array(Vec::new()),
            };
            echo((StatusCode::OK, Json(value)).into_response(), echoed)
        }
    }
}
