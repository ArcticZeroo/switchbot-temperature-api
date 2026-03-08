use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use chrono::Utc;
use hmac::{Hmac, Mac};
use reqwest::Client;
use sha2::Sha256;
use uuid::Uuid;

use crate::models::{
    DeviceInfo, SwitchBotDeviceList, SwitchBotDeviceStatus, SwitchBotResponse,
};

type HmacSha256 = Hmac<Sha256>;

const BASE_URL: &str = "https://api.switch-bot.com/v1.1";

/// Device types that have temperature/humidity sensors.
const CLIMATE_DEVICE_TYPES: &[&str] = &[
    "Hub 2",
    "Hub 3",
    "Meter",
    "Meter Plus",
    "MeterPro",
    "MeterPro(CO2)",
    "WoIOSensor",
];

#[derive(Debug, thiserror::Error)]
pub enum SwitchBotError {
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("SwitchBot API error (code {code}): {message}")]
    Api { code: i32, message: String },
}

#[derive(Clone)]
pub struct SwitchBotClient {
    client: Client,
    token: String,
    secret: String,
}

impl SwitchBotClient {
    pub fn new(token: String, secret: String) -> Self {
        Self {
            client: Client::new(),
            token,
            secret,
        }
    }

    /// Generate the required SwitchBot API auth headers.
    fn auth_headers(&self) -> Vec<(String, String)> {
        let t = Utc::now().timestamp_millis().to_string();
        let nonce = Uuid::new_v4().to_string();
        let string_to_sign = format!("{}{}{}", self.token, t, nonce);

        let mut mac =
            HmacSha256::new_from_slice(self.secret.as_bytes()).expect("HMAC key length is valid");
        mac.update(string_to_sign.as_bytes());
        let sign = BASE64.encode(mac.finalize().into_bytes());

        vec![
            ("Authorization".to_string(), self.token.clone()),
            ("sign".to_string(), sign),
            ("t".to_string(), t),
            ("nonce".to_string(), nonce),
            (
                "Content-Type".to_string(),
                "application/json; charset=utf8".to_string(),
            ),
        ]
    }

    /// List all devices that have temperature/humidity sensors.
    pub async fn discover_climate_devices(&self) -> Result<Vec<DeviceInfo>, SwitchBotError> {
        let url = format!("{}/devices", BASE_URL);

        let mut req = self.client.get(&url);
        for (key, value) in self.auth_headers() {
            req = req.header(&key, &value);
        }

        let resp: SwitchBotResponse<SwitchBotDeviceList> = req.send().await?.json().await?;

        if resp.status_code != 100 {
            return Err(SwitchBotError::Api {
                code: resp.status_code,
                message: resp.message,
            });
        }

        let devices = resp
            .body
            .device_list
            .into_iter()
            .filter(|d| CLIMATE_DEVICE_TYPES.contains(&d.device_type.as_str()))
            .map(|d| DeviceInfo {
                device_id: d.device_id,
                device_name: d.device_name,
                device_type: d.device_type,
            })
            .collect();

        Ok(devices)
    }

    /// Get current temperature/humidity status for a device.
    pub async fn get_device_status(
        &self,
        device_id: &str,
    ) -> Result<SwitchBotDeviceStatus, SwitchBotError> {
        let url = format!("{}/devices/{}/status", BASE_URL, device_id);

        let mut req = self.client.get(&url);
        for (key, value) in self.auth_headers() {
            req = req.header(&key, &value);
        }

        let resp: SwitchBotResponse<SwitchBotDeviceStatus> = req.send().await?.json().await?;

        if resp.status_code != 100 {
            return Err(SwitchBotError::Api {
                code: resp.status_code,
                message: resp.message,
            });
        }

        Ok(resp.body)
    }
}
