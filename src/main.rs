mod cli;
mod config;
mod wifi;

use crate::config::RepeaterConfig;
use anyhow::Result;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs};
use esp_idf_sys as _;
use std::sync::{Arc, Mutex};

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs_partition = EspDefaultNvsPartition::take()?;

    let nvs = EspNvs::new(nvs_partition.clone(), "repeater", true)?;
    let config = Arc::new(Mutex::new(RepeaterConfig::load(&nvs)?));

    let mut wifi_manager = wifi::WifiManager::new(peripherals.modem, sys_loop, nvs_partition)?;
    let initial_config = config.lock().unwrap().clone();
    let sta_ip = wifi_manager.start(&initial_config)?;

    let cfg_snapshot = config.lock().unwrap().clone();
    print_startup_status(&cfg_snapshot, sta_ip);

    if let Err(e) = cli::run(&mut wifi_manager, config, nvs) {
        log::error!("CLI Error: {}; entering keep-alive loop.", e);
        loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    }

    Ok(())
}

fn print_startup_status(config: &RepeaterConfig, sta_ip: std::net::Ipv4Addr) {
    log::info!("=== ESP32-C3 WiFi Repeater Ready ===");
    log::info!("STA connected to: {} | IP: {}", config.sta_ssid, sta_ip);
    log::info!(
        "AP SSID: {} | Channel: {}",
        config.ap_ssid,
        config.ap_channel
    );
    log::info!("Clients can connect to AP: {}", config.ap_ssid);
    log::info!("====================================");
}
