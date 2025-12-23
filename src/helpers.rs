use std::env;

pub enum ShellType {
    ZSH(String),
    BASH(String),
}

pub fn detect_user_shell() -> ShellType {
    let active_shell = env::var("SHELL").unwrap_or(String::from("/bin/zsh"));
    // Don't think there is a case where home dir wouldnt be defined
    // for the usecase of this program.
    let user_home_dir = env::var("HOME").unwrap_or(String::from("/home/unknown"));

    if active_shell.ends_with("bash") {
        ShellType::BASH(String::from(format!("{}/.bash_history", user_home_dir)))
    } else {
        ShellType::ZSH(String::from(format!("{}/.zsh_history", user_home_dir)))
    }
}

pub fn max_distance(cmd: &str) -> usize {
    match cmd.len() {
        0..=4 => 1,
        5..=8 => 2,
        _ => 3,
    }
}
