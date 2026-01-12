mod common;

use basket::storage::db::{
    create_db, establish_connection, get_all_history_entries, get_last_entry, get_recent_entries,
    insert_history_entry, CommandStatus,
};
use std::env;
use std::fs;

#[test]
fn db_wrapper_functions_use_configured_path() {
    let _guard = common::env_lock();
    let dir = env::temp_dir().join("basket_db_wrappers");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp dir");
    let db_path = dir.join("basket.db");

    let previous = env::var("BASKET_DB_PATH").ok();
    env::set_var("BASKET_DB_PATH", db_path.to_str().expect("db path"));

    let conn = establish_connection().expect("connect db");
    create_db(&conn).expect("create db");
    drop(conn);

    insert_history_entry(
        &1,
        "ls",
        "2024-01-01 00:00:00",
        Some(CommandStatus::Success),
        "tester",
    )
    .expect("insert entry");

    let entries = get_all_history_entries().expect("all entries");
    assert_eq!(entries.len(), 1);

    let last = get_last_entry().expect("last entry");
    assert_eq!(last.command, "ls");

    let recent = get_recent_entries(1).expect("recent entries");
    assert_eq!(recent.len(), 1);

    match previous {
        Some(val) => env::set_var("BASKET_DB_PATH", val),
        None => env::remove_var("BASKET_DB_PATH"),
    }
}
