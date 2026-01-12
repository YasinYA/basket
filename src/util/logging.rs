extern crate colored;

use colored::Colorize;
use terminal_emoji::Emoji;

pub enum Status {
    SUCCESS,
    ERROR,
    WARNING,
    INFO,
}

pub fn log_to_console(message: &str, status: Status) {
    colored::control::set_override(true);
    let success_emoji = Emoji::new("🎉", "Yay");
    let error_emoji = Emoji::new("❌", "Oops");
    let warning_emoji = Emoji::new("⚠️", "Alert");
    let info_emoji = Emoji::new("ℹ️", "info");

    let content = match status {
        Status::SUCCESS => format!("Success! {}", success_emoji).green().bold(),
        Status::ERROR => format!("Error! {}", error_emoji).red().bold(),
        Status::WARNING => format!("Warning! {}", warning_emoji).bold().yellow(),
        Status::INFO => format!("Info! {}", info_emoji).bold().cyan(),
    };

    let text = format!("{}: {}", content, message);
    println!("{}", text);
}

#[allow(dead_code)]
pub fn log_table_to_console(title: &str, emoji: Emoji, data: &[Vec<String>]) {
    println!("{} {}", emoji, title);
    for _ in 0..title.len() * 3 {
        print!("_");
    }
    println!();

    let max_len = data.iter().map(|item| item[0].len()).max().unwrap_or(0);

    println!(
        "{:<width$}           |           {:<10}",
        "Command",
        "Occurrence",
        width = max_len
    );
    for _ in 0..title.len() * 3 {
        print!("_");
    }
    println!();
    for item in data {
        println!(
            "{ :<width$}           |           {:<10}",
            item[0],
            item[1],
            width = max_len
        );
    }
}
