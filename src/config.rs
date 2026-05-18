use serde::{Deserialize, Serialize};
use esp_idf_svc::nvs::{EspNvs, NvsDefault};
use anyhow::Result;

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
            sta_ssid: "".to_string(),
            sta_password: "".to_string(),
            sta_use_static: false,
            sta_static_ip: "192.168.1.200".to_string(),
            sta_gateway: "192.168.1.1".to_string(),
            sta_netmask: "255.255.255.0".to_string(),
            ap_ssid: "ESP32-C3-Repeater".to_string(),
            ap_password: "password123".to_string(),
            ap_channel: 1,
        }
    }
}

impl RepeaterConfig {
    pub fn load(nvs: &EspNvs<NvsDefault>) -> Result<Self> {
        let mut buffer = [0u8; 512];
        match nvs.get_raw("config", &mut buffer)? {
            Some(data) => {
                let config: RepeaterConfig = serde_json::from_slice(data)?;
                Ok(config)
            }
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
