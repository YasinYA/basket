use crate::db::insert_history_entry;
use chrono::{TimeZone, Utc};
use std::fs::File;
use std::io::{BufRead, BufReader};

const HISTORY_FILE: &str = "/home/yasinya/.bash_history";

fn load_history_data() -> Result<Vec<(i64, String)>, Box<dyn std::error::Error>> {
    let file = File::open(HISTORY_FILE)?;
    let reader: BufReader<File> = BufReader::new(file);

    // Vector to hold (timestamp, command) tuples
    let mut history_entries: Vec<(i64, String)> = Vec::new();

    // Variables to store the current timestamp and command
    let mut current_timestamp: Option<i64> = None;

    for line in reader.lines() {
        match line {
            Ok(line) => {
                if line.starts_with('#') {
                    // Parse the timestamp
                    let cleaned_line = &line[1..]; // Remove the '#' character
                    if let Ok(timestamp) = cleaned_line.parse::<i64>() {
                        current_timestamp = Some(timestamp);
                    }
                } else if let Some(timestamp) = current_timestamp {
                    // Store the command and timestamp as a tuple
                    let command = line.clone(); // The current line is the command
                    history_entries.push((timestamp, command));
                    current_timestamp = None; // Reset timestamp after using it
                }
            }
            Err(err) => eprintln!("Error: {}", err),
        }
    }

    Ok(history_entries)
}

pub fn load_history() -> Result<(), Box<dyn std::error::Error>> {
    println!("Loading history from file...");
    match load_history_data() {
        Ok(history_entries) => {
            for (timestamp, command) in history_entries {
                // Safely handle potential invalid timestamps
                if let Some(datetime_utc) = Utc.timestamp_opt(timestamp, 0).single() {
                    let formatted_datetime = datetime_utc.format("%Y-%m-%d %H:%M:%S").to_string();
                    if let Err(err) = insert_history_entry(&formatted_datetime, &command) {
                        eprintln!("Failed to insert history entry: {}", err);
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
