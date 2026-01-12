use basket::analysis::get_overview_tables_from_entries;
use basket::analysis::testing::{
    calculate_command_occurrence_for_test, most_misstyped_command_for_test,
    top_5_most_unsuccessful_commands_for_test, top_5_most_used_commands_for_test,
};
use basket::storage::db::{CommandStatus, Entry};

fn entry(timestamp: i64, command: &str, status: CommandStatus) -> Entry {
    Entry {
        id: format!("id-{}", timestamp),
        timestamp,
        command: command.to_string(),
        date: "2024-01-01 00:00:00".to_string(),
        status,
        user: "tester".to_string(),
    }
}

#[test]
fn overview_tables_are_built_from_entries() {
    let entries = vec![
        entry(1, "git status", CommandStatus::Success),
        entry(2, "git status", CommandStatus::Success),
        entry(3, "git status", CommandStatus::Error(1)),
        entry(4, "alpha", CommandStatus::Success),
        entry(5, "alpha", CommandStatus::Success),
        entry(6, "beta", CommandStatus::Success),
        entry(7, "beta", CommandStatus::Success),
        entry(8, "gamma", CommandStatus::Success),
        entry(9, "gamma", CommandStatus::Success),
        entry(10, "delta", CommandStatus::Success),
        entry(11, "delta", CommandStatus::Success),
        entry(12, "epsilon", CommandStatus::Success),
        entry(13, "epsilon", CommandStatus::Success),
        entry(14, "ls -la", CommandStatus::Error(2)),
        entry(15, "ls -la", CommandStatus::Error(2)),
        entry(16, "gti status", CommandStatus::Success),
    ];

    let tables = get_overview_tables_from_entries(&entries);

    assert_eq!(tables.top_commands[0][0], "git status");
    assert_eq!(tables.top_commands[0][1], "3");
    assert_eq!(tables.top_unsuccessful[0][0], "ls -la");
    assert_eq!(tables.top_unsuccessful[0][1], "2");

    let mistyped = tables
        .mistyped_commands
        .iter()
        .find(|row| row[0] == "git status");
    assert!(mistyped.is_some());
}

#[test]
fn analysis_helpers_cover_sorting_and_counts() {
    let entries = vec![
        entry(1, "whoami", CommandStatus::Error(1)),
        entry(2, "whoami", CommandStatus::Success),
        entry(3, "whoami", CommandStatus::Success),
        entry(4, "whaomi", CommandStatus::Success),
        entry(5, "git status", CommandStatus::Success),
        entry(6, "git status", CommandStatus::Success),
        entry(7, "git status", CommandStatus::Success),
        entry(8, "ls", CommandStatus::Success),
        entry(9, "ls", CommandStatus::Success),
        entry(10, "alpha", CommandStatus::Success),
        entry(11, "alpha", CommandStatus::Success),
        entry(12, "beta", CommandStatus::Success),
        entry(13, "beta", CommandStatus::Success),
    ];

    let count = calculate_command_occurrence_for_test(&"ls".to_string(), &entries);
    assert_eq!(count, 2);

    let top = top_5_most_used_commands_for_test(&entries);
    assert!(top.iter().any(|row| row[0] == "git status"));
    assert!(top.iter().any(|row| row[0] == "whoami"));

    let unsuccessful = top_5_most_unsuccessful_commands_for_test(&entries);
    assert_eq!(unsuccessful[0][0], "whoami");

    let mistyped = most_misstyped_command_for_test(&entries);
    assert!(mistyped.iter().any(|row| row[0] == "whoami"));
}

#[test]
fn analysis_helpers_handle_empty_mistypes_and_errors() {
    let entries = vec![
        entry(1, "git status", CommandStatus::Success),
        entry(2, "git status", CommandStatus::Success),
    ];

    let mistyped = most_misstyped_command_for_test(&entries);
    assert!(mistyped.is_empty());

    let unsuccessful = top_5_most_unsuccessful_commands_for_test(&entries);
    assert!(unsuccessful.is_empty());
}

#[test]
fn mistyped_commands_sort_by_occurrence_desc() {
    let entries = vec![
        entry(1, "git status", CommandStatus::Success),
        entry(2, "git status", CommandStatus::Success),
        entry(3, "git status", CommandStatus::Success),
        entry(4, "ls", CommandStatus::Success),
        entry(5, "ls", CommandStatus::Success),
        entry(6, "ls", CommandStatus::Success),
        entry(7, "alpha", CommandStatus::Success),
        entry(8, "alpha", CommandStatus::Success),
        entry(9, "beta", CommandStatus::Success),
        entry(10, "beta", CommandStatus::Success),
        entry(11, "gamma", CommandStatus::Success),
        entry(12, "gamma", CommandStatus::Success),
        entry(13, "gti status", CommandStatus::Success),
        entry(14, "git sttaus", CommandStatus::Success),
        entry(15, "lss", CommandStatus::Success),
    ];

    let mistyped = most_misstyped_command_for_test(&entries);
    assert_eq!(mistyped[0][0], "git status");
    assert!(mistyped[0][1].starts_with("2 "));
    assert_eq!(mistyped[1][0], "ls");
    assert!(mistyped[1][1].starts_with("1 "));
}
