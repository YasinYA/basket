mod db;
mod history_file;

fn main() {
    // Connect to the database
    match db::establish_connection() {
        Ok(conn) => {
            // Create the database and table if they don't exist
            if let Err(e) = db::create_db(conn) {
                eprintln!("Failed to create database: {}", e);
                return;
            }
            // Use the connection
            println!("Database connection established.");
        }
        Err(e) => eprintln!("Failed to establish database connection: {}", e),
    }

    // Load history from the file and insert it into the database
    if let Err(e) = history_file::load_history() {
        eprintln!("Failed to load history: {}", e);
    }
}
