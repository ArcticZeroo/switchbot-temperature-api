use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ── API response types ──

#[derive(Debug, Serialize)]
pub struct ClimateReading {
    pub temperature: f64,
    pub humidity: i32,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct CurrentClimateResponse {
    pub device_id: String,
    pub temperature: f64,
    pub humidity: i32,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct HistoryResponse {
    pub device_id: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub readings: Vec<ClimateReading>,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct DeviceInfo {
    pub device_id: String,
    pub device_name: String,
    pub device_type: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

// ── SwitchBot API response types ──

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchBotResponse<T> {
    pub status_code: i32,
    pub message: String,
    pub body: T,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchBotDeviceList {
    pub device_list: Vec<SwitchBotDevice>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchBotDevice {
    pub device_id: String,
    pub device_name: String,
    pub device_type: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct SwitchBotDeviceStatus {
    pub device_id: String,
    pub device_type: String,
    pub temperature: f64,
    pub humidity: i32,
}
