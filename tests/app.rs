mod common;

use basket::app::default_deps;
use basket::app::{run_app, AppDeps};
use basket::storage::db::{create_db, ensure_user_column, establish_connection};
use basket::ui::TuiExit;
use basket::util::helpers::ShellType;
use std::env;
use std::fs;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

#[test]
fn run_app_happy_path_calls_dependencies() {
    let _guard = common::env_lock();
    let called_create = Arc::new(AtomicBool::new(false));
    let called_ensure = Arc::new(AtomicBool::new(false));
    let called_history = Arc::new(AtomicBool::new(false));
    let called_setup = Arc::new(AtomicBool::new(false));
    let called_shell = Arc::new(AtomicBool::new(false));
    let called_tui = Arc::new(AtomicBool::new(false));

    let dir = env::temp_dir().join("basket_app_happy");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp dir");
    let db_path = dir.join("basket.db");

    let deps = AppDeps {
        is_history_read: Box::new(|| false),
        establish_connection: Box::new(establish_connection),
        create_db: Box::new({
            let called_create = Arc::clone(&called_create);
            move |conn| {
                called_create.store(true, Ordering::SeqCst);
                create_db(conn)
            }
        }),
        ensure_user_column: Box::new({
            let called_ensure = Arc::clone(&called_ensure);
            move |conn| {
                called_ensure.store(true, Ordering::SeqCst);
                ensure_user_column(conn)
            }
        }),
        save_history: Box::new({
            let called_history = Arc::clone(&called_history);
            move || {
                called_history.store(true, Ordering::SeqCst);
                Ok(())
            }
        }),
        setup: Box::new({
            let called_setup = Arc::clone(&called_setup);
            move || {
                called_setup.store(true, Ordering::SeqCst);
                Ok(())
            }
        }),
        detect_user_shell: Box::new({
            let called_shell = Arc::clone(&called_shell);
            move || {
                called_shell.store(true, Ordering::SeqCst);
                ShellType::ZSH("test".to_string())
            }
        }),
        run_tui: Box::new({
            let called_tui = Arc::clone(&called_tui);
            move || {
                called_tui.store(true, Ordering::SeqCst);
                Ok(TuiExit::Exit)
            }
        }),
        log: Box::new(|_msg, _status| {}),
    };

    let previous = env::var("BASKET_DB_PATH").ok();
    env::set_var("BASKET_DB_PATH", db_path.to_str().expect("db path"));

    run_app(&deps);

    match previous {
        Some(val) => env::set_var("BASKET_DB_PATH", val),
        None => env::remove_var("BASKET_DB_PATH"),
    }

    assert!(called_create.load(Ordering::SeqCst));
    assert!(called_ensure.load(Ordering::SeqCst));
    assert!(called_history.load(Ordering::SeqCst));
    assert!(called_setup.load(Ordering::SeqCst));
    assert!(called_shell.load(Ordering::SeqCst));
    assert!(called_tui.load(Ordering::SeqCst));
}

fn with_db_env(path: &std::path::Path, f: impl FnOnce()) {
    let previous = env::var("BASKET_DB_PATH").ok();
    env::set_var("BASKET_DB_PATH", path.to_str().expect("db path"));
    f();
    match previous {
        Some(val) => env::set_var("BASKET_DB_PATH", val),
        None => env::remove_var("BASKET_DB_PATH"),
    }
}

#[test]
fn run_app_handles_connection_error_and_tui_error() {
    let _guard = common::env_lock();
    let logged = Arc::new(AtomicUsize::new(0));

    let deps = AppDeps {
        is_history_read: Box::new(|| false),
        establish_connection: Box::new(|| Err("db error".into())),
        create_db: Box::new(|_conn| Ok(())),
        ensure_user_column: Box::new(|_conn| Ok(())),
        save_history: Box::new(|| Ok(())),
        setup: Box::new(|| Ok(())),
        detect_user_shell: Box::new(|| ShellType::BASH("test".to_string())),
        run_tui: Box::new(|| Err("tui error".into())),
        log: Box::new({
            let logged = Arc::clone(&logged);
            move |_msg, _status| {
                logged.fetch_add(1, Ordering::SeqCst);
            }
        }),
    };

    run_app(&deps);
    assert!(logged.load(Ordering::SeqCst) >= 2);
}

#[test]
fn run_app_skips_history_when_already_read() {
    let _guard = common::env_lock();
    let called_history = Arc::new(AtomicBool::new(false));
    let dir = env::temp_dir().join("basket_app_skip");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp dir");
    let db_path = dir.join("basket.db");

    let deps = AppDeps {
        is_history_read: Box::new(|| true),
        establish_connection: Box::new(establish_connection),
        create_db: Box::new(|_conn| Ok(())),
        ensure_user_column: Box::new(|_conn| Ok(())),
        save_history: Box::new({
            let called_history = Arc::clone(&called_history);
            move || {
                called_history.store(true, Ordering::SeqCst);
                Ok(())
            }
        }),
        setup: Box::new(|| Ok(())),
        detect_user_shell: Box::new(|| ShellType::ZSH("test".to_string())),
        run_tui: Box::new(|| Ok(TuiExit::Exit)),
        log: Box::new(|_msg, _status| {}),
    };

    with_db_env(&db_path, || run_app(&deps));
    assert!(!called_history.load(Ordering::SeqCst));
}

#[test]
fn run_app_logs_when_create_db_fails() {
    let _guard = common::env_lock();
    let logged = Arc::new(AtomicUsize::new(0));
    let dir = env::temp_dir().join("basket_app_create_fail");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp dir");
    let db_path = dir.join("basket.db");

    let deps = AppDeps {
        is_history_read: Box::new(|| false),
        establish_connection: Box::new(establish_connection),
        create_db: Box::new(|_conn| Err("create error".into())),
        ensure_user_column: Box::new(|_conn| Ok(())),
        save_history: Box::new(|| Ok(())),
        setup: Box::new(|| Ok(())),
        detect_user_shell: Box::new(|| ShellType::ZSH("test".to_string())),
        run_tui: Box::new(|| Ok(TuiExit::Exit)),
        log: Box::new({
            let logged = Arc::clone(&logged);
            move |_msg, _status| {
                logged.fetch_add(1, Ordering::SeqCst);
            }
        }),
    };

    with_db_env(&db_path, || run_app(&deps));
    assert!(logged.load(Ordering::SeqCst) >= 1);
}

#[test]
fn run_app_logs_when_schema_update_fails() {
    let _guard = common::env_lock();
    let logged = Arc::new(AtomicUsize::new(0));
    let dir = env::temp_dir().join("basket_app_schema_fail");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp dir");
    let db_path = dir.join("basket.db");

    let deps = AppDeps {
        is_history_read: Box::new(|| false),
        establish_connection: Box::new(establish_connection),
        create_db: Box::new(|_conn| Ok(())),
        ensure_user_column: Box::new(|_conn| Err("schema error".into())),
        save_history: Box::new(|| Ok(())),
        setup: Box::new(|| Ok(())),
        detect_user_shell: Box::new(|| ShellType::ZSH("test".to_string())),
        run_tui: Box::new(|| Ok(TuiExit::Exit)),
        log: Box::new({
            let logged = Arc::clone(&logged);
            move |_msg, _status| {
                logged.fetch_add(1, Ordering::SeqCst);
            }
        }),
    };

    with_db_env(&db_path, || run_app(&deps));
    assert!(logged.load(Ordering::SeqCst) >= 1);
}

#[test]
fn run_app_logs_when_save_history_fails() {
    let _guard = common::env_lock();
    let logged = Arc::new(AtomicUsize::new(0));
    let dir = env::temp_dir().join("basket_app_history_fail");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp dir");
    let db_path = dir.join("basket.db");

    let deps = AppDeps {
        is_history_read: Box::new(|| false),
        establish_connection: Box::new(establish_connection),
        create_db: Box::new(|_conn| Ok(())),
        ensure_user_column: Box::new(|_conn| Ok(())),
        save_history: Box::new(|| Err("history error".into())),
        setup: Box::new(|| Ok(())),
        detect_user_shell: Box::new(|| ShellType::ZSH("test".to_string())),
        run_tui: Box::new(|| Ok(TuiExit::Exit)),
        log: Box::new({
            let logged = Arc::clone(&logged);
            move |_msg, _status| {
                logged.fetch_add(1, Ordering::SeqCst);
            }
        }),
    };

    with_db_env(&db_path, || run_app(&deps));
    assert!(logged.load(Ordering::SeqCst) >= 1);
}

#[test]
fn run_app_logs_when_schema_update_fails_after_history() {
    let _guard = common::env_lock();
    let logged = Arc::new(AtomicUsize::new(0));
    let dir = env::temp_dir().join("basket_app_schema_after");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp dir");
    let db_path = dir.join("basket.db");

    let deps = AppDeps {
        is_history_read: Box::new(|| true),
        establish_connection: Box::new(establish_connection),
        create_db: Box::new(|_conn| Ok(())),
        ensure_user_column: Box::new(|_conn| Err("schema error".into())),
        save_history: Box::new(|| Ok(())),
        setup: Box::new(|| Ok(())),
        detect_user_shell: Box::new(|| ShellType::ZSH("test".to_string())),
        run_tui: Box::new(|| Ok(TuiExit::Exit)),
        log: Box::new({
            let logged = Arc::clone(&logged);
            move |_msg, _status| {
                logged.fetch_add(1, Ordering::SeqCst);
            }
        }),
    };

    with_db_env(&db_path, || run_app(&deps));
    assert!(logged.load(Ordering::SeqCst) >= 1);
}

#[test]
fn default_deps_constructs() {
    let _guard = common::env_lock();
    let dir = env::temp_dir().join("basket_app_default");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp dir");
    let db_path = dir.join("basket.db");

    with_db_env(&db_path, || {
        let deps = default_deps();
        let _ = (deps.is_history_read)();
    });
}
