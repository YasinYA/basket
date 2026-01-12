use crate::util::helpers::{detect_user_shell, ShellType};
use std::io::Write;
use std::{env, fs, path};

fn ensure_zshrc_sources_hook_at(home: &str) -> std::io::Result<()> {
    let zshrc_path = path::PathBuf::from(home).join(".zshrc");
    let source_line = r#"
# basket command logger
source "$HOME/.basket.zsh"
"#;

    let contents = fs::read_to_string(&zshrc_path).unwrap_or_default();

    if !contents.contains("source \"$HOME/.basket.zsh\"") {
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(zshrc_path)?
            .write(source_line.as_bytes())?;
    }

    Ok(())
}

fn setup_zsh_with_home(home: &str) -> std::io::Result<()> {
    let hook_path = path::PathBuf::from(home).join(".basket.zsh");

    let hook_content = r#"
CMDLOG_FILE="$HOME/Documents/playground/basket/.cmdlog.json"

preexec() {
  __basket_cmd="$1"
}

precmd() {
  local exit_code=$?
  local ts=$(date +%s)
  local cmd="$__basket_cmd"
  unset __basket_cmd

  [[ -z "$cmd" ]] && return

  local cmd_escaped=${cmd//\"/\\\"}
  printf '{"cmd":"%s","status":%d, "timestamp":%d}\n' "$cmd_escaped" "$exit_code" "$ts" >> "$CMDLOG_FILE"
}
"#;

    fs::write(&hook_path, hook_content)?;
    ensure_zshrc_sources_hook_at(home)?;
    Ok(())
}

fn setup_bash_with_home(home: &str) -> std::io::Result<()> {
    let bashrc = path::PathBuf::from(home).join(".bashrc");

    let hook = r#"
# basket command logger
export PROMPT_COMMAND='
history -a
status=$?
ts=$(date +%s)
cmd=$(history 1 | sed "s/^[ ]*[0-9]\+[ ]*//")
printf "{\"cmd\":\"%s\",\"status\":%d},\"timestamp\":%d\n" "$cmd" "$status" "$ts" >> ~/.cmdlog.json
'
"#;

    let contents = fs::read_to_string(&bashrc).unwrap_or_default();
    if !contents.contains("basket command logger") {
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(bashrc)?
            .write_all(hook.as_bytes())?;
    }

    Ok(())
}

fn setup_zsh() -> std::io::Result<()> {
    let home = env::var("HOME").unwrap();
    setup_zsh_with_home(&home)
}

fn setup_bash() -> std::io::Result<()> {
    let home = env::var("HOME").unwrap();
    setup_bash_with_home(&home)
}

pub fn setup() -> std::io::Result<()> {
    match detect_user_shell() {
        ShellType::ZSH(_) => setup_zsh(),
        ShellType::BASH(_) => setup_bash(),
    }
}

pub mod testing {
    use super::{setup_bash_with_home, setup_zsh_with_home};

    pub fn setup_bash(home: &str) -> std::io::Result<()> {
        setup_bash_with_home(home)
    }

    pub fn setup_zsh(home: &str) -> std::io::Result<()> {
        setup_zsh_with_home(home)
    }
}
