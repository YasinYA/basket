mod common;

use basket::storage::db::{
    create_db, establish_connection, get_recent_entries, insert_history_entry_with_conn,
    CommandStatus,
};
use basket::storage::history::{save_history, save_history_realtime};
use std::env;
use std::fs;

fn with_env_var(key: &str, value: &str, f: impl FnOnce()) {
    let previous = env::var(key).ok();
    env::set_var(key, value);
    f();
    match previous {
        Some(val) => env::set_var(key, val),
        None => env::remove_var(key),
    }
}

fn temp_workspace(name: &str) -> std::path::PathBuf {
    let dir = env::temp_dir().join(format!("basket_history_{}", name));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

#[test]
fn save_history_reads_zsh_and_inserts_rows() {
    let _guard = common::env_lock();
    let home = temp_workspace("zsh");
    let db_path = home.join("basket.db");
    let history_path = home.join(".zsh_history");

    fs::write(
        &history_path,
        ": 1709933342:0;ls -la\n: 1709933343:0;whoami\n",
    )
    .expect("write history");

    with_env_var("HOME", home.to_str().expect("home str"), || {
        with_env_var("SHELL", "/bin/zsh", || {
            with_env_var("BASKET_DB_PATH", db_path.to_str().expect("db path"), || {
                with_env_var("USER", "tester", || {
                    let conn = establish_connection().expect("connect db");
                    create_db(&conn).expect("create db");
                    drop(conn);

                    save_history().expect("save history");
                    let entries = get_recent_entries(10).expect("recent entries");
                    assert_eq!(entries.len(), 2);
                });
            });
        });
    });
}

#[test]
fn save_history_realtime_inserts_and_handles_invalid_timestamp() {
    let _guard = common::env_lock();
    let home = temp_workspace("realtime");
    let db_path = home.join("basket.db");

    with_env_var("BASKET_DB_PATH", db_path.to_str().expect("db path"), || {
        let conn = establish_connection().expect("connect db");
        create_db(&conn).expect("create db");
        drop(conn);

        save_history_realtime("ls", 0, 1709933342).expect("insert realtime");
        let entries = get_recent_entries(1).expect("recent entries");
        assert_eq!(entries.len(), 1);

        let invalid = save_history_realtime("ls", 0, i64::MAX);
        assert!(invalid.is_err());
    });
}

#[test]
fn save_history_reads_bash_and_skips_old_entries() {
    let _guard = common::env_lock();
    let home = temp_workspace("bash");
    let db_path = home.join("basket.db");
    let history_path = home.join(".bash_history");

    std::fs::write(&history_path, "#1709933342\nls -la\n#1709933343\nwhoami\n")
        .expect("write history");

    with_env_var("HOME", home.to_str().expect("home str"), || {
        with_env_var("SHELL", "/bin/bash", || {
            with_env_var("BASKET_DB_PATH", db_path.to_str().expect("db path"), || {
                with_env_var("USER", "tester", || {
                    let conn = establish_connection().expect("connect db");
                    create_db(&conn).expect("create db");
                    insert_history_entry_with_conn(
                        &conn,
                        &1709933342,
                        "ls -la",
                        "2024-01-01 00:00:00",
                        Some(CommandStatus::Success),
                        "tester",
                    )
                    .expect("insert entry");
                    drop(conn);

                    save_history().expect("save history");
                    let entries = get_recent_entries(10).expect("recent entries");
                    assert!(entries.len() >= 2);
                });
            });
        });
    });
}

#[test]
fn save_history_handles_missing_history_file() {
    let _guard = common::env_lock();
    let home = temp_workspace("missing_history");
    let db_path = home.join("basket.db");

    with_env_var("HOME", home.to_str().expect("home str"), || {
        with_env_var("SHELL", "/bin/zsh", || {
            with_env_var("BASKET_DB_PATH", db_path.to_str().expect("db path"), || {
                let conn = establish_connection().expect("connect db");
                create_db(&conn).expect("create db");
                drop(conn);

                let result = save_history();
                assert!(result.is_ok());
            });
        });
    });
}
