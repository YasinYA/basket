use basket::storage::db::{
    create_db, get_all_history_entries_with_conn, get_last_entry_with_conn,
    get_recent_entries_with_conn, insert_history_entry_with_conn, CommandStatus,
};
use rusqlite::Connection;

fn seed_history(conn: &Connection) {
    create_db(conn).expect("create db");
    insert_history_entry_with_conn(
        conn,
        &1000,
        "ls -la",
        "2024-01-01 00:00:01",
        Some(CommandStatus::Success),
        "alice",
    )
    .expect("insert 1");
    insert_history_entry_with_conn(
        conn,
        &2000,
        "curl https://example.com",
        "2024-01-01 00:00:02",
        Some(CommandStatus::Error(2)),
        "alice",
    )
    .expect("insert 2");
    insert_history_entry_with_conn(
        conn,
        &1500,
        "whoami",
        "2024-01-01 00:00:03",
        Some(CommandStatus::Unknown),
        "bob",
    )
    .expect("insert 3");
}

#[test]
fn get_recent_entries_orders_by_timestamp() {
    let conn = Connection::open_in_memory().expect("in-memory db");
    seed_history(&conn);

    let entries = get_recent_entries_with_conn(&conn, 2).expect("recent entries");
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].timestamp, 2000);
    assert_eq!(entries[1].timestamp, 1500);
}

#[test]
fn get_last_entry_orders_by_date() {
    let conn = Connection::open_in_memory().expect("in-memory db");
    seed_history(&conn);

    let entry = get_last_entry_with_conn(&conn).expect("last entry");
    assert_eq!(entry.date, "2024-01-01 00:00:03");
    assert_eq!(entry.command, "whoami");
}

#[test]
fn get_all_entries_parses_status() {
    let conn = Connection::open_in_memory().expect("in-memory db");
    seed_history(&conn);

    let entries = get_all_history_entries_with_conn(&conn).expect("all entries");
    assert_eq!(entries.len(), 3);

    assert!(entries
        .iter()
        .any(|entry| matches!(entry.status, CommandStatus::Success)));
    assert!(entries
        .iter()
        .any(|entry| matches!(entry.status, CommandStatus::Error(2))));
    assert!(entries
        .iter()
        .any(|entry| matches!(entry.status, CommandStatus::Unknown)));
}

#[test]
fn insert_history_entry_with_conn_fails_without_table() {
    let conn = Connection::open_in_memory().expect("in-memory db");
    let result = insert_history_entry_with_conn(
        &conn,
        &1,
        "ls",
        "2024-01-01 00:00:00",
        Some(CommandStatus::Success),
        "alice",
    );
    assert!(result.is_err());
}

#[test]
fn get_all_entries_handles_unknown_status_strings() {
    let conn = Connection::open_in_memory().expect("in-memory db");
    create_db(&conn).expect("create db");
    conn.execute(
        "INSERT INTO history (id, timestamp, command, date, status, user) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            "bad-status",
            3000i64,
            "strange",
            "2024-01-01 00:00:04",
            "Error(not-a-number)",
            "root"
        ],
    )
    .expect("insert bad status");
    conn.execute(
        "INSERT INTO history (id, timestamp, command, date, status, user) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            "weird-status",
            4000i64,
            "odd",
            "2024-01-01 00:00:05",
            "Weird",
            "root"
        ],
    )
    .expect("insert weird status");

    let entries = get_all_history_entries_with_conn(&conn).expect("all entries");
    assert!(entries
        .iter()
        .any(|entry| matches!(entry.status, CommandStatus::Error(-1))));
    assert!(entries
        .iter()
        .any(|entry| matches!(entry.status, CommandStatus::Unknown)));
}
