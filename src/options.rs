use std::io::{self, Write};

use crate::logging::{log_to_console, Status};

#[derive(Debug, Copy, Clone)]
pub enum ProgramOption {
    Overview,
    Realtime,
    Exit,
}

pub fn prompt_user_option() -> ProgramOption {
    loop {
        log_to_console("Basket menu", Status::INFO);
        println!("--------------------------");
        println!("1) View overview");
        println!("2) Start realtime watcher");
        println!("3) Exit");
        println!("--------------------------");
        print!("Enter choice (1-3): ");

        let _ = io::stdout().flush();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            println!("Failed to read input. Try again.");
            continue;
        }

        match input.trim() {
            "1" => return ProgramOption::Overview,
            "2" => return ProgramOption::Realtime,
            "3" => return ProgramOption::Exit,
            _ => {
                log_to_console("Invalid choice. Please enter 1, 2, or 3.", Status::WARNING);
            }
        }
    }
}
