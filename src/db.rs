use rusqlite::{params, Connection, Result};
use std::error::Error;
use uuid::Uuid;

const DB_FILE: &str = "/Users/yasinya/basket.db";

#[allow(dead_code)]
#[derive(Debug)]
pub struct Entry {
    pub id: String,
    pub timestamp: i64,
    pub command: String,
    pub date: String,
}

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

pub fn create_db(conn: &Connection) -> Result<(), Box<dyn Error>> {
    // Attempt to create the table
    if let Err(e) = conn.execute(
        "CREATE TABLE IF NOT EXISTS history (
            id TEXT PRIMARY KEY,
            timestamp INTERGER NOT NULL,
            date TEXT NOT NULL,
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
pub fn insert_history_entry(
    timestamp: &i64,
    command: &str,
    date: &str,
) -> Result<(), Box<dyn Error>> {
    match establish_connection() {
        Ok(conn) => {
            let id = Uuid::new_v4();
            // Attempt to insert data into the table
            if let Err(e) = conn.execute(
                "INSERT INTO history (id, timestamp, command, date) VALUES (?1, ?2, ?3, ?4)",
                params![id.to_string(), timestamp, command, date],
            ) {
                eprintln!("Failed to insert history entry: {}", e);
                return Err(Box::new(e));
            }
        }
        Err(e) => eprintln!("Failed to establish database connection: {}", e),
    }

    Ok(())
}

fn _get_all_history_entries() -> Result<(), Box<dyn Error>> {
    match establish_connection() {
        Ok(conn) => {
            let mut stmt = conn.prepare("SELECT id, timestamp, command, date FROM history")?;

            let rows = stmt.query_map([], |row| {
                Ok(Entry {
                    id: row.get::<_, String>(0)?,
                    timestamp: row.get::<_, i64>(1)?,
                    command: row.get::<_, String>(2)?,
                    date: row.get::<_, String>(3)?,
                })
            })?;

            for x in rows {
                let entry = x?;
                println!("id: {}", entry.id);
                println!("timestamp: {}", entry.timestamp);
                println!("command: {}", entry.command);
            }
        }
        Err(e) => eprintln!("Failed to establish the database connection: {}", e),
    }

    Ok(())
}

pub fn get_last_entry() -> Result<Entry, Box<dyn Error>> {
    let conn = establish_connection()?;

    let mut stmt = conn
        .prepare("SELECT id, timestamp, command, date FROM history ORDER BY date DESC LIMIT 1")?;
    let mut last_row = stmt.query_map([], |row| {
        Ok(Entry {
            id: row.get(0)?,
            timestamp: row.get::<_, i64>(1)?,
            command: row.get(2)?,
            date: row.get(3)?,
        })
    })?;
    if let Some(result) = last_row.next() {
        let entry = result?;
        return Ok(entry);
    } else {
        return Err("No entries found".into());
    }
}

// pub fn edit_entry(id: String, data: Vec<(i64, String)>) -> Result<(), Box<dyn Error>> {
//     Ok(())
// }

// pub fn delete_entry(id: String) -> Result<(), Box<dyn Error>> {
//     Ok(())
// }
