use crate::runtime::init::setup;
use crate::storage::db;
use crate::storage::history;
use crate::ui::{run as run_tui, TuiExit};
use crate::util::helpers::{detect_user_shell, is_history_read, ShellType};
use crate::util::logging::{log_to_console, Status};
use rusqlite::Connection;
use std::error::Error;

pub struct AppDeps {
    pub is_history_read: Box<dyn Fn() -> bool>,
    pub establish_connection: Box<dyn Fn() -> Result<Connection, Box<dyn Error>>>,
    pub create_db: Box<dyn Fn(&Connection) -> Result<(), Box<dyn Error>>>,
    pub ensure_user_column: Box<dyn Fn(&Connection) -> Result<(), Box<dyn Error>>>,
    pub save_history: Box<dyn Fn() -> Result<(), Box<dyn Error>>>,
    pub setup: Box<dyn Fn() -> std::io::Result<()>>,
    pub detect_user_shell: Box<dyn Fn() -> ShellType>,
    pub run_tui: Box<dyn Fn() -> Result<TuiExit, Box<dyn Error>>>,
    pub log: Box<dyn Fn(&str, Status)>,
}

pub fn default_deps() -> AppDeps {
    AppDeps {
        is_history_read: Box::new(is_history_read),
        establish_connection: Box::new(db::establish_connection),
        create_db: Box::new(db::create_db),
        ensure_user_column: Box::new(db::ensure_user_column),
        save_history: Box::new(history::save_history),
        setup: Box::new(setup),
        detect_user_shell: Box::new(detect_user_shell),
        run_tui: Box::new(run_tui),
        log: Box::new(log_to_console),
    }
}

pub fn run_app(deps: &AppDeps) {
    if !(deps.is_history_read)() {
        match (deps.establish_connection)() {
            Ok(conn) => {
                if let Err(e) = (deps.create_db)(&conn) {
                    (deps.log)(&format!("Failed to create database: {}", e), Status::ERROR);
                    return;
                }
                if let Err(e) = (deps.ensure_user_column)(&conn) {
                    (deps.log)(&format!("Failed to update schema: {}", e), Status::ERROR);
                    return;
                }
                (deps.log)("Database connection established.", Status::SUCCESS);
            }
            Err(e) => (deps.log)(
                &format!("Failed to establish database connection: {}", e),
                Status::ERROR,
            ),
        }
        if let Err(e) = (deps.save_history)() {
            (deps.log)(&format!("Failed to load history: {}", e), Status::ERROR);
        }
    }

    if let Ok(conn) = (deps.establish_connection)() {
        if let Err(e) = (deps.ensure_user_column)(&conn) {
            (deps.log)(&format!("Failed to update schema: {}", e), Status::ERROR);
        }
    }

    let _ = (deps.setup)();
    (deps.detect_user_shell)();

    match (deps.run_tui)() {
        Ok(TuiExit::Exit) => {}
        Err(err) => {
            (deps.log)(&format!("TUI failed: {}", err), Status::ERROR);
        }
    }
}
