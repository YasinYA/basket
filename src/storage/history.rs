use crate::storage::db::{get_last_entry, insert_history_entry, CommandStatus, Entry};
use chrono::{TimeZone, Utc};
use std::fs::File;
use std::io::{BufRead, BufReader};
use uuid::Uuid;

use crate::util::helpers::{current_user, detect_user_shell, ShellType};
use crate::util::logging::{log_to_console, Status};

fn load_history_from_zsh(
    line: &str,
    history_entries: &mut Vec<(i64, String)>,
    current_timestamp: &mut Option<i64>,
) {
    if line.starts_with(':') {
        // Example line: ": 1709933342:0;ls -la"
        let parts: Vec<&str> = line.splitn(2, ';').collect();
        if let Some(meta) = parts.get(0) {
            // meta = ": 1709933342:0"
            if let Some(ts_str) = meta.trim_start_matches(": ").split(':').next() {
                if let Ok(timestamp) = ts_str.parse::<i64>() {
                    *current_timestamp = Some(timestamp);
                    // Command part (after ';')
                    let command = parts.get(1).unwrap_or(&"").trim().to_string();
                    history_entries.push((timestamp, command));
                }
            }
        }
    } else if let Some(&ts) = current_timestamp.as_ref() {
        // Continuation of previous command (same timestamp)
        history_entries.push((ts, line.to_string()));
    } else {
        // If we somehow get a continuation before any timestamp (rare)
        history_entries.push((0, line.to_string()));
    }
}

fn load_history_from_bash(
    line: &str,
    history_entries: &mut Vec<(i64, String)>,
    current_timestamp: &mut Option<i64>,
) {
    if line.starts_with('#') {
        // Parse timestamp line (e.g., "#1709933342")
        if let Ok(ts) = line[1..].trim().parse::<i64>() {
            *current_timestamp = Some(ts);
        }
    } else if let Some(ts) = current_timestamp.take() {
        // The previous line was a timestamp — this one is the command
        history_entries.push((ts, line.to_string()));
    }
}

pub fn parse_zsh_line(
    line: &str,
    history_entries: &mut Vec<(i64, String)>,
    current_timestamp: &mut Option<i64>,
) {
    load_history_from_zsh(line, history_entries, current_timestamp);
}

pub fn parse_bash_line(
    line: &str,
    history_entries: &mut Vec<(i64, String)>,
    current_timestamp: &mut Option<i64>,
) {
    load_history_from_bash(line, history_entries, current_timestamp);
}

fn load_history_data() -> Result<Vec<(i64, String)>, Box<dyn std::error::Error>> {
    let active_shell = detect_user_shell();
    let parser: fn(&str, &mut Vec<(i64, String)>, &mut Option<i64>) = match active_shell {
        ShellType::ZSH(_) => load_history_from_zsh,
        ShellType::BASH(_) => load_history_from_bash,
    };

    // Extract the history file path from the enum
    let history_file_path = match &active_shell {
        ShellType::ZSH(path) | ShellType::BASH(path) => path,
    };

    let file = File::open(history_file_path)?;
    let reader: BufReader<File> = BufReader::new(file);

    // Vector to hold (timestamp, command) tuples
    let mut history_entries: Vec<(i64, String)> = Vec::new();
    let mut current_timestamp: Option<i64> = None;

    for line in reader.lines() {
        match line {
            Ok(line) => {
                parser(&line, &mut history_entries, &mut current_timestamp);
            }
            Err(err) => log_to_console(&err.to_string(), Status::ERROR),
        }
    }

    Ok(history_entries)
}

pub fn save_history() -> Result<(), Box<dyn std::error::Error>> {
    let user = current_user();
    let last_entry = match get_last_entry() {
        Ok(entry) => entry,
        Err(e) => {
            log_to_console(
                &format!("Warning: couldn't fetch last row: {}", e),
                Status::WARNING,
            );
            Entry {
                id: Uuid::new_v4().to_string(),
                timestamp: 0,
                command: String::new(),
                date: Utc.to_string(),
                status: crate::storage::db::CommandStatus::Unknown,
                user: String::new(),
            }
        }
    };

    log_to_console("Loading history from file...", Status::INFO);
    if last_entry.timestamp == 0 {
        log_to_console("Fresh DB", Status::INFO);
    } else {
        log_to_console("Inserted only unsaved entries.", Status::INFO);
    }
    match load_history_data() {
        Ok(history_entries) => {
            for (timestamp, command) in history_entries {
                // Safely handle potential invalid timestamps
                if let Some(datetime_utc) = Utc.timestamp_opt(timestamp, 0).single() {
                    let formatted_datetime = datetime_utc.format("%Y-%m-%d %H:%M:%S").to_string();
                    if timestamp > last_entry.timestamp {
                        if let Err(err) = insert_history_entry(
                            &timestamp,
                            &command,
                            &formatted_datetime,
                            Some(crate::storage::db::CommandStatus::Unknown),
                            &user,
                        ) {
                            log_to_console(
                                &format!("Failed to insert history entry: {}", err),
                                Status::ERROR,
                            )
                        }
                    }
                } else {
                    log_to_console(&format!("Invalid timestamp: {}", timestamp), Status::ERROR);
                }
            }
            log_to_console("Successfully inserted history", Status::SUCCESS);
        }
        Err(err) => {
            log_to_console(&format!("Failed to load history: {}", err), Status::ERROR);
        }
    }

    Ok(())
}

pub fn save_history_realtime(
    command: &str,
    status: i32,
    timestamp: i64,
) -> Result<(), Box<dyn std::error::Error>> {
    let user = current_user();
    // Convert UNIX timestamp → UTC datetime
    let datetime_utc = Utc
        .timestamp_opt(timestamp, 0)
        .single()
        .ok_or("Invalid timestamp")?;

    let formatted_datetime = datetime_utc.format("%Y-%m-%d %H:%M:%S").to_string();

    // Map exit code → CommandStatus
    let command_status = match status {
        0 => Some(CommandStatus::Success),
        code if code > 0 => Some(CommandStatus::Error(code)),
        _ => Some(CommandStatus::Unknown),
    };

    // Insert into DB
    if let Err(err) = insert_history_entry(
        &timestamp,
        command,
        &formatted_datetime,
        command_status,
        &user,
    ) {
        log_to_console(
            &format!("Failed to insert history entry: {}", err),
            Status::ERROR,
        );
    }

    Ok(())
}
