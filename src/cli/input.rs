use super::commands;
use super::CommandContext;
use anyhow::Result;
use std::io::{self, Write};

macro_rules! print_flush {
    ($($arg:tt)*) => {{
        print!($($arg)*);
        io::stdout().flush()
    }};
}

const RECONNECT_CHECK_TICKS: u32 = 100;
const RECONNECT_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(100);

pub fn run(context: &mut CommandContext) -> Result<()> {
    let mut line = String::new();
    let mut ticks = 0;

    print_prompt()?;

    loop {
        let c = unsafe { esp_idf_sys::getchar() };
        if c == -1 {
            poll_reconnect(context, &mut ticks);
            continue;
        }

        ticks = 0;
        handle_input_byte(c as u8, &mut line, context)?;
    }
}

fn poll_reconnect(context: &mut CommandContext, ticks: &mut u32) {
    std::thread::sleep(RECONNECT_POLL_INTERVAL);
    *ticks += 1;

    if *ticks < RECONNECT_CHECK_TICKS {
        return;
    }

    *ticks = 0;
    if context.wifi_manager.is_connected().unwrap_or(false) {
        return;
    }

    log::warn!("WiFi disconnected. Attempting to reconnect...");
    match context.wifi_manager.reconnect() {
        Ok(new_ip) => log::info!("Reconnected successfully. IP: {}", new_ip),
        Err(e) => log::error!("Reconnection failed: {}", e),
    }
}

fn handle_input_byte(byte: u8, line: &mut String, context: &mut CommandContext) -> Result<()> {
    match byte {
        b'\n' | b'\r' => submit_line(line, context),
        8 | 127 => erase_last_char(line),
        _ => append_char(byte as char, line),
    }
}

fn submit_line(line: &mut String, context: &mut CommandContext) -> Result<()> {
    println!();

    let trimmed = line.trim();
    if !trimmed.is_empty() {
        let parts = split_command(trimmed);
        commands::run(&parts, context);
    }

    line.clear();
    print_prompt()
}

fn split_command(line: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut remaining = line;

    for _ in 0..2 {
        let Some(pos) = remaining.find(char::is_whitespace) else {
            break;
        };
        parts.push(&remaining[..pos]);
        remaining = remaining[pos..].trim_start();
    }

    if !remaining.is_empty() {
        parts.push(remaining);
    }

    parts
}

fn erase_last_char(line: &mut String) -> Result<()> {
    if !line.is_empty() {
        line.pop();
        print_flush!("{}{}{}", 8 as char, ' ', 8 as char)?;
    }
    Ok(())
}

fn append_char(ch: char, line: &mut String) -> Result<()> {
    line.push(ch);
    print_flush!("{}", ch)?;
    Ok(())
}

fn print_prompt() -> Result<()> {
    Ok(print_flush!("> ")?)
}
