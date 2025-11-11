use terminal_emoji::Emoji;

use crate::db::{get_all_history_entries, Entry};
use crate::logging::{log_table_to_console, log_to_console, Status};

pub fn overview_analysis() {
    let mut data: Vec<Vec<String>> = Vec::new();
    match get_all_history_entries() {
        Ok(entries) => {
            for entry in entries.iter() {
                let occurance = calculate_command_occurance(&entry.command, &entries);
                data.push(vec![entry.command.clone(), occurance.to_string()]);
            }
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
