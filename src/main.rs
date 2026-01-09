mod analyze_commands;
mod db;
mod helpers;
mod history_file;
mod init;
mod logging;
mod options;
mod realtime_commands;

use analyze_commands::overview_analysis;
use helpers::{detect_user_shell, is_history_read};
use init::setup;
use logging::{log_to_console, Status};
use options::{prompt_user_option, ProgramOption};
use realtime_commands::save_realtime_commands;

fn main() {
    println!(
        r#"
    _               _        _
   | |             | |      | |
   | |__   __ _ ___| | _____| |_
   | '_ \ / _` / __| |/ / _ \ __|
   | |_) | (_| \__ \   <  __/ |_
   |_.__/ \__,_|___/_|\_\___|\__|

       "#
    );
    // Only run this for the first time the program is run
    // we know this by check if th db is empty
    // Load history from the file and insert it into the database
    if !is_history_read() {
        // Connect to the database
        match db::establish_connection() {
            Ok(conn) => {
                // Create the database and table if they don't exist
                if let Err(e) = db::create_db(&conn) {
                    log_to_console(&format!("Failed to create database: {}", e), Status::ERROR);
                    return;
                }
                log_to_console("Database connection established.", Status::SUCCESS);
            }
            Err(e) => log_to_console(
                &format!("Failed to establish database connection: {}", e),
                Status::ERROR,
            ),
        }
        if let Err(e) = history_file::save_history() {
            log_to_console(&format!("Failed to load history: {}", e), Status::ERROR);
        }
    }

    let _ = setup();
    detect_user_shell();

    match prompt_user_option() {
        ProgramOption::Overview => {
            overview_analysis();
        }
        ProgramOption::Realtime => {
            let _ = save_realtime_commands();
        }
        ProgramOption::Exit => {}
    }
}
