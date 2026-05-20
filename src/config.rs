use anyhow::Result;
use esp_idf_svc::nvs::{EspNvs, NvsDefault};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepeaterConfig {
    pub sta_ssid: String,
    pub sta_password: String,
    pub sta_use_static: bool,
    pub sta_static_ip: String,
    pub sta_gateway: String,
    pub sta_netmask: String,
    pub ap_ssid: String,
    pub ap_password: String,
    pub ap_channel: u8,
}

impl Default for RepeaterConfig {
    fn default() -> Self {
        Self {
            sta_ssid: option_env!("REPEATER_STA_SSID").unwrap_or("").to_string(),
            sta_password: option_env!("REPEATER_STA_PASS").unwrap_or("").to_string(),
            sta_use_static: option_env!("REPEATER_STA_USE_STATIC").map_or(false, |v| v == "true"),
            sta_static_ip: option_env!("REPEATER_STA_STATIC_IP").unwrap_or("192.168.10.2").to_string(),
            sta_gateway: option_env!("REPEATER_STA_GATEWAY").unwrap_or("192.168.10.1").to_string(),
            sta_netmask: option_env!("REPEATER_STA_NETMASK").unwrap_or("255.255.255.0").to_string(),
            ap_ssid: option_env!("REPEATER_AP_SSID").unwrap_or("ESP32-C3-Repeater").to_string(),
            ap_password: option_env!("REPEATER_AP_PASS").unwrap_or("").to_string(),
            ap_channel: option_env!("REPEATER_AP_CHANNEL").and_then(|v| v.parse().ok()).unwrap_or(1),
        }
    }
}

impl RepeaterConfig {
    pub fn load(nvs: &EspNvs<NvsDefault>) -> Result<Self> {
        let mut buffer = [0u8; 512];
        match nvs.get_raw("config", &mut buffer)? {
            Some(data) => match serde_json::from_slice::<RepeaterConfig>(data) {
                Ok(config) => Ok(config),
                Err(e) => {
                    log::warn!("Config parse error: {}, using defaults.", e);
                    Ok(Self::default())
                }
            },
            None => {
                log::warn!("Config not found in NVS, using defaults.");
                Ok(Self::default())
            }
        }
    }

    pub fn save(&self, nvs: &mut EspNvs<NvsDefault>) -> Result<()> {
        let data = serde_json::to_vec(self)?;
        nvs.set_raw("config", &data)?;
        Ok(())
    }

}
