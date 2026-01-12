mod common;

use basket::analysis::overview_analysis;
use basket::storage::db::{
    create_db, establish_connection, insert_history_entry_with_conn, CommandStatus,
};
use std::env;
use std::fs;

#[test]
fn overview_analysis_reads_from_db() {
    let _guard = common::env_lock();
    let dir = env::temp_dir().join("basket_analysis_db");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp dir");
    let db_path = dir.join("basket.db");

    let previous = env::var("BASKET_DB_PATH").ok();
    env::set_var("BASKET_DB_PATH", db_path.to_str().expect("db path"));

    let conn = establish_connection().expect("connect db");
    create_db(&conn).expect("create db");
    insert_history_entry_with_conn(
        &conn,
        &1,
        "ls",
        "2024-01-01 00:00:00",
        Some(CommandStatus::Success),
        "tester",
    )
    .expect("insert entry");
    insert_history_entry_with_conn(
        &conn,
        &2,
        "ls",
        "2024-01-01 00:00:01",
        Some(CommandStatus::Error(1)),
        "tester",
    )
    .expect("insert entry");

    overview_analysis();

    match previous {
        Some(val) => env::set_var("BASKET_DB_PATH", val),
        None => env::remove_var("BASKET_DB_PATH"),
    }
}

#[test]
fn overview_analysis_logs_on_db_error() {
    let _guard = common::env_lock();
    let dir = env::temp_dir().join("basket_analysis_db_err");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp dir");

    let previous = env::var("BASKET_DB_PATH").ok();
    env::set_var("BASKET_DB_PATH", dir.to_str().expect("dir path"));

    overview_analysis();

    match previous {
        Some(val) => env::set_var("BASKET_DB_PATH", val),
        None => env::remove_var("BASKET_DB_PATH"),
    }
}
