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
