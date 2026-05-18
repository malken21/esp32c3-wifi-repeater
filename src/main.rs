mod config;
mod wifi;
mod napt;

use anyhow::Result;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs, NvsDefault};
use esp_idf_sys as _;
use std::sync::{Arc, Mutex};
use std::io::{self, Write};
use crate::config::RepeaterConfig;


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

    napt::enable_napt(u32::from(sta_ip).to_be());
    log::info!("Connected. IP: {}", sta_ip);

    if let Err(e) = run_command_interface(config, nvs) {
        log::error!("CLI Error: {}", e);
    }

    Ok(())
}

fn run_command_interface(config: Arc<Mutex<RepeaterConfig>>, mut nvs: EspNvs<NvsDefault>) -> Result<()> {
    let stdin = io::stdin();
    
    loop {
        print!("> ");
        io::stdout().flush()?;

        let mut line = String::new();
        if stdin.read_line(&mut line)? == 0 {
            break;
        }

        let parts: Vec<&str> = line.trim().split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0].to_uppercase().as_str() {
            "HELP" => print_help(),
            "GET" => {
                let cfg = config.lock().unwrap();
                println!("{:#?}", *cfg);
            }
            "SET" => handle_set_command(&parts, &config),
            "SAVE" => {
                let cfg = config.lock().unwrap();
                if let Err(e) = cfg.save(&mut nvs) {
                    println!("Error: {}", e);
                } else {
                    println!("Saved.");
                }
            }
            "RESTART" => {
                println!("Restarting...");
                unsafe { esp_idf_sys::esp_restart() };
            }

            _ => println!("Unknown command. Type HELP."),
        }
    }

    Ok(())
}

fn print_help() {
    println!("Commands: HELP, GET, SET <key> <val>, SAVE, RESTART");
    println!("Keys: sta_ssid, sta_pass, sta_static, sta_ip, sta_gw, sta_nm, ap_ssid, ap_pass, ap_chan");
}

fn handle_set_command(parts: &[&str], config: &Arc<Mutex<RepeaterConfig>>) {
    if parts.len() < 3 {
        println!("Usage: SET <key> <value>");
        return;
    }

    let key = parts[1];
    let val = parts[2];
    let mut cfg = config.lock().unwrap();

    match key {
        "sta_ssid" => cfg.sta_ssid = val.to_string(),
        "sta_pass" => cfg.sta_password = val.to_string(),
        "sta_static" => cfg.sta_use_static = val.parse().unwrap_or(false),
        "sta_ip" => cfg.sta_static_ip = val.to_string(),
        "sta_gw" => cfg.sta_gateway = val.to_string(),
        "sta_nm" => cfg.sta_netmask = val.to_string(),
        "ap_ssid" => cfg.ap_ssid = val.to_string(),
        "ap_pass" => cfg.ap_password = val.to_string(),
        "ap_chan" => cfg.ap_channel = val.parse().unwrap_or(6),
        _ => println!("Unknown key: {}", key),
    }
}


