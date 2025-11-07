mod db;
mod helpers;
mod history_file;
mod logging;

use helpers::detect_user_shell;
use logging::{log_to_console, Status};

fn main() {
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

    // Load history from the file and insert it into the database
    if let Err(e) = history_file::save_history() {
        log_to_console(&format!("Failed to load history: {}", e), Status::ERROR);
    }

    detect_user_shell();
}
