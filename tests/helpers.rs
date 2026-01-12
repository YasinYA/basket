mod common;

use basket::storage::db::{
    create_db, establish_connection, insert_history_entry_with_conn, CommandStatus,
};
use basket::util::helpers::{
    current_user, detect_user_shell, is_history_read, max_distance, ShellType,
};
use std::env;
use std::path::PathBuf;

fn with_env<K: AsRef<str>, V: AsRef<str>, F: FnOnce()>(key: K, value: V, f: F) {
    let key = key.as_ref();
    let previous = env::var(key).ok();
    env::set_var(key, value.as_ref());
    f();
    match previous {
        Some(val) => env::set_var(key, val),
        None => env::remove_var(key),
    }
}

#[test]
fn detects_bash_history_path() {
    let _guard = common::env_lock();
    let home = env::temp_dir().join("basket_helpers_bash");
    let home_str = home.to_string_lossy().to_string();

    with_env("HOME", &home_str, || {
        with_env("SHELL", "/bin/bash", || match detect_user_shell() {
            ShellType::BASH(path) => {
                assert_eq!(PathBuf::from(path), home.join(".bash_history"));
            }
            _ => panic!("expected bash shell"),
        });
    });
}

#[test]
fn detects_zsh_history_path() {
    let _guard = common::env_lock();
    let home = env::temp_dir().join("basket_helpers_zsh");
    let home_str = home.to_string_lossy().to_string();

    with_env("HOME", &home_str, || {
        with_env("SHELL", "/bin/zsh", || match detect_user_shell() {
            ShellType::ZSH(path) => {
                assert_eq!(PathBuf::from(path), home.join(".zsh_history"));
            }
            _ => panic!("expected zsh shell"),
        });
    });
}

#[test]
fn max_distance_scales_with_length() {
    assert_eq!(max_distance("ls"), 1);
    assert_eq!(max_distance("abcdef"), 2);
    assert_eq!(max_distance("verylongcommand"), 3);
}

#[test]
fn current_user_prefers_user_env() {
    let _guard = common::env_lock();
    with_env("USER", "basket-user", || {
        with_env("LOGNAME", "basket-log", || {
            assert_eq!(current_user(), "basket-user");
        });
    });
}

#[test]
fn current_user_falls_back_to_logname() {
    let _guard = common::env_lock();
    let prev_user = env::var("USER").ok();
    env::remove_var("USER");
    with_env("LOGNAME", "basket-log", || {
        assert_eq!(current_user(), "basket-log");
    });
    match prev_user {
        Some(val) => env::set_var("USER", val),
        None => env::remove_var("USER"),
    }
}

#[test]
fn current_user_defaults_to_unknown() {
    let _guard = common::env_lock();
    let prev_user = env::var("USER").ok();
    let prev_logname = env::var("LOGNAME").ok();
    env::remove_var("USER");
    env::remove_var("LOGNAME");
    assert_eq!(current_user(), "unknown");
    match prev_user {
        Some(val) => env::set_var("USER", val),
        None => env::remove_var("USER"),
    }
    match prev_logname {
        Some(val) => env::set_var("LOGNAME", val),
        None => env::remove_var("LOGNAME"),
    }
}

#[test]
fn detect_user_shell_defaults_to_zsh() {
    let _guard = common::env_lock();
    let prev_shell = env::var("SHELL").ok();
    let prev_home = env::var("HOME").ok();
    env::remove_var("SHELL");
    env::set_var("HOME", "/tmp");
    match detect_user_shell() {
        ShellType::ZSH(path) => assert!(path.ends_with("/tmp/.zsh_history")),
        _ => panic!("expected zsh shell"),
    }
    match prev_shell {
        Some(val) => env::set_var("SHELL", val),
        None => env::remove_var("SHELL"),
    }
    match prev_home {
        Some(val) => env::set_var("HOME", val),
        None => env::remove_var("HOME"),
    }
}

#[test]
fn is_history_read_tracks_empty_and_non_empty_db() {
    let _guard = common::env_lock();
    let home = env::temp_dir().join("basket_helpers_db");
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).expect("create temp dir");
    let db_path = home.join("basket.db");

    with_env("BASKET_DB_PATH", db_path.to_str().expect("db path"), || {
        let conn = establish_connection().expect("connect");
        create_db(&conn).expect("create db");
        drop(conn);

        assert!(!is_history_read());

        let conn = establish_connection().expect("connect");
        insert_history_entry_with_conn(
            &conn,
            &1,
            "ls",
            "2024-01-01 00:00:00",
            Some(CommandStatus::Success),
            "tester",
        )
        .expect("insert entry");
        drop(conn);

        assert!(is_history_read());
    });
}
