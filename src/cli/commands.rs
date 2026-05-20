use super::CommandContext;
use crate::config::RepeaterConfig;
use std::io;
use std::net::{SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};

const HTTP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

const HELP_COMMANDS: &str =
    "Commands: HELP, GET, SET <key> <val>, SAVE, RESTART, CURL <host>, PING";
const HELP_KEYS: &str =
    "Keys: sta_ssid, sta_pass, sta_static, sta_ip, sta_gw, sta_nm, ap_ssid, ap_pass, ap_chan";
const HELP_PING: &str = "PING: TCP connectivity test to 1.1.1.1, 8.8.8.8, example.com (port 80, no DNS)";
const HELP_CURL: &str = "CURL <host>: Send HTTP GET request to the specified host";

pub fn run(parts: &[&str], context: &mut CommandContext) {
    match parts[0].to_uppercase().as_str() {
        "HELP" => print_help(),
        "GET" => print_config(&context.config),
        "SET" => set_config_value(parts, &context.config),
        "SAVE" => save_config(context),
        "RESTART" => restart_device(),
        "CURL" => handle_curl_command(parts),
        "PING" => handle_ping_command(),
        _ => println!("Unknown command. Type HELP."),
    }
}

fn print_help() {
    println!("{}", HELP_COMMANDS);
    println!("{}", HELP_KEYS);
    println!("{}", HELP_PING);
    println!("{}", HELP_CURL);
}

fn print_config(config: &Arc<Mutex<RepeaterConfig>>) {
    let cfg = config.lock().unwrap();
    println!("{:#?}", *cfg);
}

fn save_config(context: &mut CommandContext) {
    let cfg = context.config.lock().unwrap();
    if let Err(e) = cfg.save(context.nvs) {
        println!("Error: {}", e);
    } else {
        println!("Saved.");
    }
}

fn restart_device() {
    println!("Restarting...");
    unsafe { esp_idf_sys::esp_restart() };
}

fn set_config_value(parts: &[&str], config: &Arc<Mutex<RepeaterConfig>>) {
    if parts.len() < 3 {
        println!("Usage: SET <key> <value>");
        return;
    }

    let key = parts[1];
    let value = unquote(parts[2]);
    let mut cfg = config.lock().unwrap();

    match key {
        "sta_ssid" => set_string(
            &mut cfg.sta_ssid,
            value,
            32,
            "STA SSID",
            "sta_ssid updated.",
        ),
        "sta_pass" => set_string(
            &mut cfg.sta_password,
            value,
            64,
            "STA Password",
            "sta_pass updated.",
        ),
        "sta_static" => set_bool(&mut cfg.sta_use_static, &value),
        "sta_ip" => set_ipv4(&mut cfg.sta_static_ip, value, "sta_ip updated."),
        "sta_gw" => set_ipv4(&mut cfg.sta_gateway, value, "sta_gw updated."),
        "sta_nm" => set_ipv4(&mut cfg.sta_netmask, value, "sta_nm updated."),
        "ap_ssid" => set_string(&mut cfg.ap_ssid, value, 32, "AP SSID", "ap_ssid updated."),
        "ap_pass" => set_ap_password(&mut cfg.ap_password, value),
        "ap_chan" => set_ap_channel(&mut cfg.ap_channel, &value),
        _ => println!("Unknown key: {}", key),
    }
}

fn unquote(raw: &str) -> String {
    if raw.starts_with('"') && raw.ends_with('"') && raw.len() >= 2 {
        raw[1..raw.len() - 1].to_string()
    } else {
        raw.to_string()
    }
}

fn set_string(target: &mut String, value: String, max_len: usize, label: &str, success: &str) {
    if value.len() > max_len {
        println!("Error: {} too long (max {} chars)", label, max_len);
    } else {
        *target = value;
        println!("{}", success);
    }
}

fn set_bool(target: &mut bool, value: &str) {
    if let Ok(parsed) = value.parse::<bool>() {
        *target = parsed;
        println!("sta_static updated.");
    } else {
        println!("Error: sta_static must be true or false");
    }
}

fn set_ipv4(target: &mut String, value: String, success: &str) {
    if value.parse::<std::net::Ipv4Addr>().is_ok() {
        *target = value;
        println!("{}", success);
    } else {
        println!("Error: Invalid IPv4 address format");
    }
}

fn set_ap_password(target: &mut String, value: String) {
    if !value.is_empty() && (value.len() < 8 || value.len() > 64) {
        println!("Error: AP password must be empty or between 8 and 64 chars.");
    } else {
        *target = value;
        println!("ap_pass updated.");
    }
}

fn set_ap_channel(target: &mut u8, value: &str) {
    match value.parse::<u8>() {
        Ok(channel) if (1..=13).contains(&channel) => {
            *target = channel;
            println!("ap_chan updated.");
        }
        Ok(_) => println!("Error: Channel must be between 1 and 13"),
        Err(_) => println!("Error: Invalid channel number"),
    }
}

fn handle_curl_command(parts: &[&str]) {
    if parts.len() < 2 {
        println!("Usage: CURL <host>");
        return;
    }

    let host = parts[1];
    println!("Connecting to {} on port 80...", host);

    match open_http_stream(host) {
        Ok(mut stream) => request_http_root(host, &mut stream),
        Err(e) => println!("Failed to connect to {}: {}", host, e),
    }
}

fn open_http_stream(host: &str) -> io::Result<TcpStream> {
    let addr = format!("{}:80", host);
    match addr.parse::<SocketAddr>() {
        Ok(socket_addr) => TcpStream::connect_timeout(&socket_addr, HTTP_TIMEOUT),
        Err(_) => TcpStream::connect(addr),
    }
}

fn request_http_root(host: &str, stream: &mut TcpStream) {
    let _ = stream.set_read_timeout(Some(HTTP_TIMEOUT));
    let _ = stream.set_write_timeout(Some(HTTP_TIMEOUT));
    println!("Connected! Sending HTTP request...");

    let request = format!(
        "GET / HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        host
    );

    if let Err(e) = std::io::Write::write_all(stream, request.as_bytes()) {
        println!("Failed to send request: {}", e);
        return;
    }

    read_http_response(stream);
}

fn read_http_response(stream: &mut TcpStream) {
    println!("Reading response...");

    let mut response = [0u8; 512];
    match std::io::Read::read(stream, &mut response) {
        Ok(bytes_read) if bytes_read > 0 => {
            print_response_preview(&response[..bytes_read]);
            println!("[OK] External access successful");
        }
        Ok(_) => println!("Received empty response."),
        Err(e) => println!("Failed to read response: {}", e),
    }
}

fn print_response_preview(response: &[u8]) {
    let text = String::from_utf8_lossy(response);
    println!("--- Response (first {} bytes) ---", response.len());
    println!("{}", text);
    println!("---------------------------------");
}

fn handle_ping_command() {
    println!("=== Network Connectivity Test ===");

    test_tcp_target("1.1.1.1:80", "Cloudflare");
    test_tcp_target("8.8.8.8:80", "Google");
    test_tcp_target("93.184.216.34:80", "example.com");

    println!("=================================");
}

fn test_tcp_target(addr: &str, label: &str) {
    print!("  Testing {} [{}]... ", label, addr);

    let socket_addr = addr.parse::<SocketAddr>().unwrap();
    match TcpStream::connect_timeout(&socket_addr, HTTP_TIMEOUT) {
        Ok(_) => println!("OK"),
        Err(e) => println!("FAIL ({})", e),
    }
}
