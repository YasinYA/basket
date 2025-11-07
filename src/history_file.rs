use crate::db::{get_last_entry, insert_history_entry, Entry};
use chrono::{TimeZone, Utc};
use std::fs::File;
use std::io::{BufRead, BufReader};
use uuid::Uuid;

// TODO: Move this Contants to args
const HISTORY_FILE: &str = "/Users/yasinya/.zsh_history";
const SHELL: &str = "ZSH";

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

fn load_history_data() -> Result<Vec<(i64, String)>, Box<dyn std::error::Error>> {
    let file = File::open(HISTORY_FILE)?;
    let reader: BufReader<File> = BufReader::new(file);

    // Vector to hold (timestamp, command) tuples
    let mut history_entries: Vec<(i64, String)> = Vec::new();
    let mut current_timestamp: Option<i64> = None;

    for line in reader.lines() {
        match line {
            Ok(line) => {
                if SHELL == "ZSH" {
                    load_history_from_zsh(&line, &mut history_entries, &mut current_timestamp);
                } else {
                    load_history_from_bash(&line, &mut history_entries, &mut current_timestamp);
                }
            }
            Err(err) => eprintln!("Error: {}", err),
        }
    }

    Ok(history_entries)
}

pub fn save_history() -> Result<(), Box<dyn std::error::Error>> {
    let last_entry = match get_last_entry() {
        Ok(entry) => entry,
        Err(e) => {
            eprintln!("Warning: couldn't fetch last row: {}", e);
            Entry {
                id: Uuid::new_v4().to_string(),
                timestamp: 0,
                command: String::new(),
                date: Utc.to_string(),
            }
        }
    };

    println!("Loading history from file...");
    if last_entry.timestamp == 0 {
        println!("Fresh DB");
    } else {
        println!("Inserted only unsaved entries.");
    }
    match load_history_data() {
        Ok(history_entries) => {
            for (timestamp, command) in history_entries {
                // Safely handle potential invalid timestamps
                if let Some(datetime_utc) = Utc.timestamp_opt(timestamp, 0).single() {
                    let formatted_datetime = datetime_utc.format("%Y-%m-%d %H:%M:%S").to_string();
                    if timestamp > last_entry.timestamp {
                        if let Err(err) =
                            insert_history_entry(&timestamp, &command, &formatted_datetime)
                        {
                            eprintln!("Failed to insert history entry: {}", err);
                        }
                    }
                } else {
                    eprintln!("Invalid timestamp: {}", timestamp);
                }
            }
        }
        Err(err) => {
            eprintln!("Failed to load history: {}", err);
        }
    }

    Ok(())
}
