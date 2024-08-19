use rusqlite::{params, Connection, Result};
use std::error::Error;

const DB_FILE: &str = "src/basket.db";

pub fn establish_connection() -> Result<Connection, Box<dyn Error>> {
    // Attempt to open the connection to the database
    let conn = match Connection::open(DB_FILE) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to connect to the database: {}", e);
            return Err(Box::new(e));
        }
    };
    Ok(conn)
}

pub fn create_db(conn: Connection) -> Result<(), Box<dyn Error>> {
    // Attempt to create the table
    if let Err(e) = conn.execute(
        "CREATE TABLE IF NOT EXISTS history (
            id INTEGER PRIMARY KEY,
            timestamp TEXT NOT NULL,
            command TEXT NOT NULL
        )",
        [],
    ) {
        eprintln!("Failed to create table: {}", e);
        return Err(Box::new(e));
    }

    println!("Database tables created successfully.");
    Ok(())
}

// Function to insert a history entry into the table
pub fn insert_history_entry(timestamp: &str, command: &str) -> Result<(), Box<dyn Error>> {
    match establish_connection() {
        Ok(conn) => {
            // Attempt to insert data into the table
            if let Err(e) = conn.execute(
                "INSERT INTO history (timestamp, command) VALUES (?1, ?2)",
                params![timestamp, command],
            ) {
                eprintln!("Failed to insert history entry: {}", e);
                return Err(Box::new(e));
            }
        }
        Err(e) => eprintln!("Failed to establish database connection: {}", e),
    }

    Ok(())
}
