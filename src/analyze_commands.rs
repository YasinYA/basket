use std::collections::{HashMap, HashSet};
use strsim::levenshtein;
use terminal_emoji::Emoji;

use crate::db::{get_all_history_entries, Entry};
use crate::helpers::max_distance;
use crate::logging::{log_table_to_console, log_to_console, Status};

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

fn most_misstyped_command(entries: &Vec<Entry>) -> Vec<Vec<String>> {
    // Extract command names only
    let common_commands: Vec<String> = top_5_most_used_commands(entries)
        .into_iter()
        .filter_map(|row| row.get(0).cloned())
        .collect();

    let mut stats: HashMap<String, (usize, HashSet<String>)> = HashMap::new();

    for entry in entries {
        let cmd = &entry.command;

        if common_commands.contains(cmd) {
            continue;
        }

        for common in &common_commands {
            let dist = levenshtein(cmd, common);

            if dist <= max_distance(cmd) {
                let entry = stats.entry(common.clone()).or_insert((0, HashSet::new()));

                entry.0 += 1;
                entry.1.insert(cmd.clone());
            }
        }
    }

    // Convert to display rows
    let mut rows: Vec<Vec<String>> = stats
        .into_iter()
        .map(|(correct, (count, variants))| {
            let variants = variants.into_iter().collect::<Vec<_>>().join(", ");

            vec![correct, format!("{} ({})", count, variants)]
        })
        .collect();

    // Sort by occurrence desc
    rows.sort_by(|a, b| {
        let a_count = a[1]
            .split_whitespace()
            .next()
            .unwrap()
            .parse::<usize>()
            .unwrap();
        let b_count = b[1]
            .split_whitespace()
            .next()
            .unwrap()
            .parse::<usize>()
            .unwrap();
        b_count.cmp(&a_count)
    });

    rows
}

pub fn overview_analysis() {
    match get_all_history_entries() {
        Ok(entries) => {
            let top_commands = top_5_most_used_commands(&entries);
            let mistyped_commands = most_misstyped_command(&entries);

            log_table_to_console(
                "Command Occurrences",
                Emoji::new("🔢", "Occurrence"),
                &top_commands,
            );
            println!("\n\n");
            log_table_to_console(
                "Most Mistyped Commands",
                Emoji::new("💬", "Miss Type"),
                &mistyped_commands,
            );
        }
        Err(e) => {
            log_to_console(
                &format!("Failed to get history entries: {}", e),
                Status::ERROR,
            );
        }
    }
}
