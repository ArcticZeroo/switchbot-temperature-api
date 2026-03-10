use axum::{
    Router,
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
    Json,
};
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use std::sync::Arc;

use crate::db::Database;
use crate::models::{
    CurrentClimateResponse, ErrorResponse, HealthResponse, HistoryResponse,
};
use crate::switchbot::SwitchBotClient;

const MAX_RANGE_DAYS: i64 = 7;

pub struct AppState {
    pub db: Arc<Database>,
    pub switchbot: SwitchBotClient,
    pub device_id: String,
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(stats_page))
        .route("/health", get(health))
        .route("/api/climate/current", get(current_climate))
        .route("/api/climate/history", get(climate_history))
        .with_state(state)
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

async fn stats_page() -> Html<&'static str> {
    Html(include_str!("stats.html"))
}

async fn current_climate(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let status = state
        .switchbot
        .get_device_status(&state.device_id)
        .await
        .map_err(|e| {
            tracing::error!("SwitchBot API error: {}", e);
            (
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse {
                    error: format!("upstream error: {}", e),
                }),
            )
        })?;

    Ok(Json(CurrentClimateResponse {
        device_id: state.device_id.clone(),
        temperature: status.temperature,
        humidity: status.humidity,
        timestamp: Utc::now(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

async fn climate_history(
    State(state): State<Arc<AppState>>,
    Query(query): Query<HistoryQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Validate time range
    let range = query.end - query.start;
    if range > Duration::days(MAX_RANGE_DAYS) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("time range must not exceed {} days", MAX_RANGE_DAYS),
            }),
        ));
    }
    if query.start > query.end {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "start must be before end".to_string(),
            }),
        ));
    }

    let readings = state
        .db
        .query_readings(&state.device_id, query.start, query.end)
        .map_err(|e| {
            tracing::error!("database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "database query failed".to_string(),
                }),
            )
        })?;

    let count = readings.len();
    Ok(Json(HistoryResponse {
        device_id: state.device_id.clone(),
        start: query.start,
        end: query.end,
        readings,
        count,
    }))
}
