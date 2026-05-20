mod commands;
mod input;

use crate::config::RepeaterConfig;
use crate::wifi;
use anyhow::Result;
use esp_idf_svc::nvs::{EspNvs, NvsDefault};
use std::sync::{Arc, Mutex};

pub fn run(
    wifi_manager: &mut wifi::WifiManager,
    config: Arc<Mutex<RepeaterConfig>>,
    mut nvs: EspNvs<NvsDefault>,
) -> Result<()> {
    let mut context = CommandContext {
        wifi_manager,
        config,
        nvs: &mut nvs,
    };
    input::run(&mut context)
}

pub(super) struct CommandContext<'a, 'b> {
    pub wifi_manager: &'a mut wifi::WifiManager<'b>,
    pub config: Arc<Mutex<RepeaterConfig>>,
    pub nvs: &'a mut EspNvs<NvsDefault>,
}
