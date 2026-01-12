use basket::util::logging::{log_table_to_console, log_to_console, Status};
use terminal_emoji::Emoji;

#[test]
fn log_to_console_covers_all_statuses() {
    log_to_console("ok", Status::SUCCESS);
    log_to_console("no", Status::ERROR);
    log_to_console("warn", Status::WARNING);
    log_to_console("info", Status::INFO);
}

#[test]
fn log_table_to_console_renders_rows() {
    let data = vec![
        vec!["ls".to_string(), "2".to_string()],
        vec!["git".to_string(), "1".to_string()],
    ];
    log_table_to_console("Commands", Emoji::new("📦", "box"), &data);
}
