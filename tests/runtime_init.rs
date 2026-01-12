mod common;

use basket::runtime::init::setup;
use basket::runtime::init::testing::{setup_bash, setup_zsh};
use std::env;
use std::fs;

fn temp_home_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("basket_{}", name));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

#[test]
fn setup_zsh_writes_hook_and_sources() {
    let home = temp_home_dir("zsh_init");
    setup_zsh(home.to_str().expect("home string")).expect("setup zsh");

    let hook_path = home.join(".basket.zsh");
    let zshrc_path = home.join(".zshrc");

    let hook = fs::read_to_string(hook_path).expect("read hook");
    assert!(hook.contains("CMDLOG_FILE"));

    let zshrc = fs::read_to_string(zshrc_path).expect("read zshrc");
    assert!(zshrc.contains("source \"$HOME/.basket.zsh\""));
}

#[test]
fn setup_bash_writes_prompt_command() {
    let home = temp_home_dir("bash_init");
    setup_bash(home.to_str().expect("home string")).expect("setup bash");

    let bashrc_path = home.join(".bashrc");
    let bashrc = fs::read_to_string(bashrc_path).expect("read bashrc");
    assert!(bashrc.contains("basket command logger"));
    assert!(bashrc.contains("PROMPT_COMMAND"));
}

#[test]
fn setup_uses_zsh_when_shell_matches() {
    let _guard = common::env_lock();
    let home = temp_home_dir("setup_zsh");
    let home_str = home.to_str().expect("home string");

    let prev_home = env::var("HOME").ok();
    let prev_shell = env::var("SHELL").ok();
    env::set_var("HOME", home_str);
    env::set_var("SHELL", "/bin/zsh");

    setup().expect("setup");

    let hook_path = home.join(".basket.zsh");
    assert!(hook_path.exists());

    match prev_home {
        Some(val) => env::set_var("HOME", val),
        None => env::remove_var("HOME"),
    }
    match prev_shell {
        Some(val) => env::set_var("SHELL", val),
        None => env::remove_var("SHELL"),
    }
}

#[test]
fn setup_uses_bash_when_shell_matches() {
    let _guard = common::env_lock();
    let home = temp_home_dir("setup_bash");
    let home_str = home.to_str().expect("home string");

    let prev_home = env::var("HOME").ok();
    let prev_shell = env::var("SHELL").ok();
    env::set_var("HOME", home_str);
    env::set_var("SHELL", "/bin/bash");

    setup().expect("setup");

    let bashrc_path = home.join(".bashrc");
    assert!(bashrc_path.exists());

    match prev_home {
        Some(val) => env::set_var("HOME", val),
        None => env::remove_var("HOME"),
    }
    match prev_shell {
        Some(val) => env::set_var("SHELL", val),
        None => env::remove_var("SHELL"),
    }
}

#[test]
fn setup_zsh_is_idempotent() {
    let home = temp_home_dir("zsh_idempotent");
    let home_str = home.to_str().expect("home string");
    setup_zsh(home_str).expect("setup zsh 1");
    setup_zsh(home_str).expect("setup zsh 2");

    let zshrc_path = home.join(".zshrc");
    let zshrc = fs::read_to_string(zshrc_path).expect("read zshrc");
    assert_eq!(zshrc.matches("source \"$HOME/.basket.zsh\"").count(), 1);
}

#[test]
fn setup_bash_is_idempotent() {
    let home = temp_home_dir("bash_idempotent");
    let home_str = home.to_str().expect("home string");
    setup_bash(home_str).expect("setup bash 1");
    setup_bash(home_str).expect("setup bash 2");

    let bashrc_path = home.join(".bashrc");
    let bashrc = fs::read_to_string(bashrc_path).expect("read bashrc");
    assert_eq!(bashrc.matches("basket command logger").count(), 1);
}
