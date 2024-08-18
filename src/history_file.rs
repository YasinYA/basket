use chrono::{TimeZone, Utc};
use std::fs::File;
use std::io::{BufRead, BufReader};

const HISTORY_FILE: &str = "src/.bash_history";

pub fn load_history() -> Result<String, Box<dyn std::error::Error>> {
    println!("Loading history.......");
    let file = File::open(HISTORY_FILE)?;
    let reader: BufReader<File> = BufReader::new(file);
    let mut lines: Vec<String> = Vec::new();

    for line in reader.lines() {
        match line {
            Ok(line) => {
                if line.starts_with('#') {
                    // Remove the # character and parse the timestamp
                    let cleaned_lined = &line[1..];
                    let timestamp: i64 = cleaned_lined.parse()?;

                    // Adjust date format as per your file
                    let datetime_utc = Utc
                        .timestamp_opt(timestamp, 0)
                        .single()
                        .ok_or("Invalid date")?;

                    lines.push(datetime_utc.format("%Y-%m-%d %H:%M:%S").to_string());
                } else {
                    lines.push(line);
                }
            }
            Err(err) => eprintln!("Error: {}", err),
        }
    }

    Ok(lines.join("\n"))
}
