use std::collections::HashMap;
use terminal_emoji::Emoji;

use crate::db::{get_all_history_entries, Entry};
use crate::logging::{log_table_to_console, log_to_console, Status};

pub fn overview_analysis() {
    let mut data: Vec<Vec<String>> = Vec::new();
    match get_all_history_entries() {
        Ok(entries) => {
            data = top_5_most_used_commands(&entries);
        }
        Err(e) => {
            log_to_console(
                &format!("Failed to get history entries: {}", e),
                Status::ERROR,
            );
        }
    }
    log_table_to_console("Command Occurances", Emoji::new("🔢", "Occurance"), &data);
}

fn calculate_command_occurance(command: &String, entries: &Vec<Entry>) -> i32 {
    let mut count: i32 = 0;

    for entry in entries.iter() {
        if entry.command == *command {
            count += 1;
        }
    }
    count
}

fn top_5_most_used_commands(entries: &Vec<Entry>) -> Vec<Vec<String>> {
    let mut top_5: Vec<Vec<String>> = Vec::new();
    let mut occurrences: HashMap<String, i32> = HashMap::new();

    for entry in entries {
        let count = calculate_command_occurance(&entry.command, entries);
        occurrences.insert(entry.command.clone(), count);
    }

    let mut sorted: Vec<(String, i32)> = occurrences.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));

    // Take the top 5 and convert to Vec<Vec<String>>
    for (cmd, count) in sorted.into_iter().take(5) {
        top_5.push(vec![cmd, count.to_string()]);
    }

    top_5
}
