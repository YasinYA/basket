use crate::util::logging::{log_to_console, Status};
use rusqlite::{params, Connection, Result};
use std::error::Error;
use uuid::Uuid;

const DEFAULT_DB_FILE: &str = "/Users/yasinya/basket.db";

fn db_path() -> String {
    std::env::var("BASKET_DB_PATH").unwrap_or_else(|_| DEFAULT_DB_FILE.to_string())
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum CommandStatus {
    Success,
    Error(i32),
    Unknown,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct Entry {
    pub id: String,
    pub timestamp: i64,
    pub command: String,
    pub date: String,
    pub status: CommandStatus,
    pub user: String,
}

pub fn establish_connection() -> Result<Connection, Box<dyn Error>> {
    // Attempt to open the connection to the database
    let conn = match Connection::open(db_path()) {
        Ok(c) => c,
        Err(e) => {
            log_to_console(
                &format!("Failed to connect to the database: {}", e),
                Status::ERROR,
            );
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
    timestamp INTEGER NOT NULL,
    date TEXT NOT NULL,
    command TEXT NOT NULL,
    status TEXT NOT NULL,
    user TEXT NOT NULL
    );
    ",
        [],
    ) {
        log_to_console(&format!("Failed to create table: {}", e), Status::ERROR);
        return Err(Box::new(e));
    }
    log_to_console("Database tables created successfully.", Status::SUCCESS);
    Ok(())
}

// Function to insert a history entry into the table
pub fn insert_history_entry(
    timestamp: &i64,
    command: &str,
    date: &str,
    status: Option<CommandStatus>,
    user: &str,
) -> Result<(), Box<dyn Error>> {
    let status = status.unwrap_or(CommandStatus::Unknown);
    match establish_connection() {
        Ok(conn) => {
            insert_history_entry_with_conn(&conn, timestamp, command, date, Some(status), user)?;
        }
        Err(e) => log_to_console(
            &format!("Failed to establish database connection: {}", e),
            Status::ERROR,
        ),
    }

    Ok(())
}

pub fn insert_history_entry_with_conn(
    conn: &Connection,
    timestamp: &i64,
    command: &str,
    date: &str,
    status: Option<CommandStatus>,
    user: &str,
) -> Result<(), Box<dyn Error>> {
    let status = status.unwrap_or(CommandStatus::Unknown);
    let id = Uuid::new_v4();
    if let Err(e) = conn.execute(
        "INSERT INTO history (id, timestamp, command, date, status, user) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            id.to_string(),
            timestamp,
            command,
            date,
            format!("{:?}", status),
            user
        ],
    ) {
        log_to_console(
            &format!("Failed to insert history entry: {}", e),
            Status::ERROR,
        );
        eprintln!();
        return Err(Box::new(e));
    }
    Ok(())
}

fn parse_status(s: &str) -> CommandStatus {
    match s {
        "Success" => CommandStatus::Success,
        "Unknown" => CommandStatus::Unknown,
        _ if s.starts_with("Error(") => {
            let code = s
                .trim_start_matches("Error(")
                .trim_end_matches(")")
                .parse::<i32>()
                .unwrap_or(-1);
            CommandStatus::Error(code)
        }
        _ => CommandStatus::Unknown,
    }
}

pub fn get_all_history_entries() -> Result<Vec<Entry>, Box<dyn Error>> {
    let conn = establish_connection()?;
    get_all_history_entries_with_conn(&conn)
}

pub fn get_all_history_entries_with_conn(conn: &Connection) -> Result<Vec<Entry>, Box<dyn Error>> {
    let mut stmt =
        conn.prepare("SELECT id, timestamp, command, date, status, user FROM history")?;

    let entries_iter = stmt.query_map([], |row| {
        let status_str: String = row.get(4)?;
        Ok(Entry {
            id: row.get::<_, String>(0)?,
            timestamp: row.get::<_, i64>(1)?,
            command: row.get::<_, String>(2)?,
            date: row.get::<_, String>(3)?,
            status: parse_status(&status_str),
            user: row.get::<_, String>(5)?,
        })
    })?;

    let entries: Result<Vec<Entry>, _> = entries_iter.collect();

    Ok(entries?)
}

pub fn get_last_entry() -> Result<Entry, Box<dyn Error>> {
    let conn = establish_connection()?;

    get_last_entry_with_conn(&conn)
}

pub fn get_last_entry_with_conn(conn: &Connection) -> Result<Entry, Box<dyn Error>> {
    let mut stmt = conn.prepare(
        "SELECT id, timestamp, command, date, status, user FROM history ORDER BY date DESC LIMIT 1",
    )?;
    let mut last_row = stmt.query_map([], |row| {
        let status_str: String = row.get(4)?;
        Ok(Entry {
            id: row.get(0)?,
            timestamp: row.get::<_, i64>(1)?,
            command: row.get(2)?,
            date: row.get(3)?,
            status: parse_status(&status_str),
            user: row.get::<_, String>(5)?,
        })
    })?;
    if let Some(result) = last_row.next() {
        let entry = result?;
        return Ok(entry);
    } else {
        log_to_console("No entries found", Status::ERROR);
        return Err("No entries found".into());
    }
}

pub fn get_recent_entries(limit: i64) -> Result<Vec<Entry>, Box<dyn Error>> {
    let conn = establish_connection()?;
    get_recent_entries_with_conn(&conn, limit)
}

pub fn get_recent_entries_with_conn(
    conn: &Connection,
    limit: i64,
) -> Result<Vec<Entry>, Box<dyn Error>> {
    let mut stmt = conn.prepare(
        "SELECT id, timestamp, command, date, status, user FROM history ORDER BY timestamp DESC LIMIT ?1",
    )?;

    let entries_iter = stmt.query_map([limit], |row| {
        let status_str: String = row.get(4)?;
        Ok(Entry {
            id: row.get::<_, String>(0)?,
            timestamp: row.get::<_, i64>(1)?,
            command: row.get::<_, String>(2)?,
            date: row.get::<_, String>(3)?,
            status: parse_status(&status_str),
            user: row.get::<_, String>(5)?,
        })
    })?;

    let entries: Result<Vec<Entry>, _> = entries_iter.collect();
    Ok(entries?)
}

pub fn ensure_user_column(conn: &Connection) -> Result<(), Box<dyn Error>> {
    let mut stmt = conn.prepare("PRAGMA table_info(history)")?;
    let mut rows = stmt.query([])?;
    let mut has_user = false;
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name == "user" {
            has_user = true;
            break;
        }
    }
    if !has_user {
        conn.execute(
            "ALTER TABLE history ADD COLUMN user TEXT NOT NULL DEFAULT 'unknown'",
            [],
        )?;
    }
    Ok(())
}
