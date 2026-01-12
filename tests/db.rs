mod common;

use basket::storage::db::{
    create_db, ensure_user_column, establish_connection, get_last_entry, get_recent_entries,
    insert_history_entry, CommandStatus,
};
use rusqlite::Connection;
use std::env;

#[test]
fn ensure_user_column_adds_missing_column() {
    let conn = Connection::open_in_memory().expect("in-memory db");
    conn.execute(
        "CREATE TABLE history (
            id TEXT PRIMARY KEY,
            timestamp INTEGER NOT NULL,
            date TEXT NOT NULL,
            command TEXT NOT NULL,
            status TEXT NOT NULL
        );",
        [],
    )
    .expect("create legacy table");

    ensure_user_column(&conn).expect("ensure user column");

    let mut stmt = conn.prepare("PRAGMA table_info(history)").expect("pragma");
    let mut rows = stmt.query([]).expect("query");
    let mut has_user = false;
    while let Some(row) = rows.next().expect("row") {
        let name: String = row.get(1).expect("name");
        if name == "user" {
            has_user = true;
            break;
        }
    }
    assert!(has_user);
}

#[test]
fn ensure_user_column_noops_when_present() {
    let conn = Connection::open_in_memory().expect("in-memory db");
    conn.execute(
        "CREATE TABLE history (
            id TEXT PRIMARY KEY,
            timestamp INTEGER NOT NULL,
            date TEXT NOT NULL,
            command TEXT NOT NULL,
            status TEXT NOT NULL,
            user TEXT NOT NULL
        );",
        [],
    )
    .expect("create table");

    ensure_user_column(&conn).expect("ensure user column");
}

fn with_env(key: &str, value: &str, f: impl FnOnce()) {
    let previous = env::var(key).ok();
    env::set_var(key, value);
    f();
    match previous {
        Some(val) => env::set_var(key, val),
        None => env::remove_var(key),
    }
}

#[test]
fn establish_connection_errors_with_directory_path() {
    let _guard = common::env_lock();
    let dir = env::temp_dir().join("basket_db_dir");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create dir");
    let dir_str = dir.to_str().expect("dir str");

    with_env("BASKET_DB_PATH", dir_str, || {
        let result = establish_connection();
        assert!(result.is_err());
    });
}

#[test]
fn insert_history_entry_handles_connection_error() {
    let _guard = common::env_lock();
    let dir = env::temp_dir().join("basket_db_insert_err");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create dir");
    let dir_str = dir.to_str().expect("dir str");

    with_env("BASKET_DB_PATH", dir_str, || {
        let result = insert_history_entry(
            &1,
            "ls",
            "2024-01-01 00:00:00",
            Some(CommandStatus::Success),
            "tester",
        );
        assert!(result.is_ok());
    });
}

#[test]
fn get_last_entry_errors_on_empty_table() {
    let _guard = common::env_lock();
    let dir = env::temp_dir().join("basket_db_empty");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create dir");
    let db_path = dir.join("basket.db");

    with_env("BASKET_DB_PATH", db_path.to_str().expect("db path"), || {
        let conn = establish_connection().expect("connect db");
        create_db(&conn).expect("create db");
        drop(conn);
        assert!(get_last_entry().is_err());
    });
}

#[test]
fn get_recent_entries_empty_returns_empty_vec() {
    let _guard = common::env_lock();
    let dir = env::temp_dir().join("basket_db_recent_empty");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create dir");
    let db_path = dir.join("basket.db");

    with_env("BASKET_DB_PATH", db_path.to_str().expect("db path"), || {
        let conn = establish_connection().expect("connect db");
        create_db(&conn).expect("create db");
        drop(conn);
        let entries = get_recent_entries(5).expect("recent entries");
        assert!(entries.is_empty());
    });
}
