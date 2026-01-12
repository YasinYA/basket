use basket::storage::history::{parse_bash_line, parse_zsh_line};

#[test]
fn parse_zsh_history_lines() {
    let mut entries: Vec<(i64, String)> = Vec::new();
    let mut current: Option<i64> = None;

    parse_zsh_line(": 1709933342:0;ls -la", &mut entries, &mut current);
    parse_zsh_line("echo second", &mut entries, &mut current);

    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].0, 1709933342);
    assert_eq!(entries[0].1, "ls -la");
    assert_eq!(entries[1].0, 1709933342);
    assert_eq!(entries[1].1, "echo second");
}

#[test]
fn parse_bash_history_lines() {
    let mut entries: Vec<(i64, String)> = Vec::new();
    let mut current: Option<i64> = None;

    parse_bash_line("#1709933342", &mut entries, &mut current);
    parse_bash_line("ls -la", &mut entries, &mut current);

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].0, 1709933342);
    assert_eq!(entries[0].1, "ls -la");
    assert!(current.is_none());
}

#[test]
fn parse_zsh_line_without_timestamp_uses_default() {
    let mut entries: Vec<(i64, String)> = Vec::new();
    let mut current: Option<i64> = None;

    parse_zsh_line("orphan command", &mut entries, &mut current);

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].0, 0);
    assert_eq!(entries[0].1, "orphan command");
}

#[test]
fn parse_bash_line_ignores_when_no_timestamp() {
    let mut entries: Vec<(i64, String)> = Vec::new();
    let mut current: Option<i64> = None;

    parse_bash_line("ls -la", &mut entries, &mut current);

    assert!(entries.is_empty());
}

#[test]
fn parse_zsh_line_ignores_malformed_timestamp() {
    let mut entries: Vec<(i64, String)> = Vec::new();
    let mut current: Option<i64> = None;

    parse_zsh_line(": not-a-ts:0;ls", &mut entries, &mut current);

    assert!(entries.is_empty());
}
