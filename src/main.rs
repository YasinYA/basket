mod history_file;

fn main() {
    match history_file::load_history() {
        Ok(history) => println!("{}", history),
        Err(e) => eprintln!("Error: {}", e),
    }
}
