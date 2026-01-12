use serde::Deserialize;
use std::collections::VecDeque;
use std::fs;
use std::path::Path;

use crate::storage::db::{CommandStatus, Entry};

#[derive(Debug, Clone)]
pub struct IntrusionFinding {
    pub command: String,
    pub score: i32,
    pub reasons: Vec<String>,
    pub timestamp: i64,
    pub user: String,
    pub user_type: String,
}

#[derive(Debug, Clone)]
struct RuleMatch {
    id: &'static str,
    score: i32,
    reason: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CmdTag {
    Recon,
    Download,
    Exec,
    PrivEsc,
}

#[derive(Debug, Default, Deserialize)]
struct IdsConfig {
    #[serde(default)]
    suppress_rules: Vec<String>,
    #[serde(default)]
    allowlist_commands: Vec<String>,
}

pub fn detect_intrusions(entries: &[Entry]) -> Vec<IntrusionFinding> {
    let config = load_ids_config();
    let current_user = detect_current_user();
    let mut findings = Vec::new();

    let mut sorted: Vec<&Entry> = entries.iter().collect();
    sorted.sort_by_key(|e| e.timestamp);

    let mut window: VecDeque<(i64, Vec<CmdTag>)> = VecDeque::new();
    let window_secs = 300;

    for entry in sorted {
        let raw = entry.command.trim();
        if raw.is_empty() {
            continue;
        }
        let raw_lower = raw.to_lowercase();
        if is_allowlisted(&raw_lower, &config) {
            continue;
        }

        let mut matches = Vec::new();
        let tags = classify_tags(&raw_lower);

        window.push_back((entry.timestamp, tags.clone()));
        while let Some((ts, _)) = window.front() {
            if entry.timestamp.saturating_sub(*ts) > window_secs {
                window.pop_front();
            } else {
                break;
            }
        }

        matches.extend(rule_high_risk(&raw_lower));
        matches.extend(rule_lolbins(&raw_lower));
        matches.extend(rule_history_tamper(&raw_lower));
        matches.extend(rule_persistence(&raw_lower));
        matches.extend(rule_recon(&raw_lower));
        matches.extend(rule_priv_esc(&raw_lower));
        matches.extend(rule_remote_exec(&raw_lower));

        if tags.contains(&CmdTag::Exec) && sequence_recon_download_exec(&window) {
            matches.push(RuleMatch {
                id: "seq.recon_download_exec",
                score: 10,
                reason: "recon → download → exec sequence",
            });
        }

        if tags.contains(&CmdTag::Exec) && sequence_sudo_shell(&window) {
            matches.push(RuleMatch {
                id: "seq.sudo_shell",
                score: 6,
                reason: "sudo → shell sequence",
            });
        }

        let matches = apply_suppression(matches, &config);
        if matches.is_empty() {
            continue;
        }

        let score: i32 = matches.iter().map(|m| m.score).sum();
        let reasons = matches.iter().map(|m| m.reason.to_string()).collect();

        let entry_user = if entry.user.is_empty() {
            current_user.clone()
        } else {
            entry.user.clone()
        };
        let user_type = classify_user(&entry_user);

        findings.push(IntrusionFinding {
            command: raw.to_string(),
            score,
            reasons,
            timestamp: entry.timestamp,
            user: entry_user,
            user_type: user_type.to_string(),
        });
    }

    findings.sort_by(|a, b| b.score.cmp(&a.score).then(b.timestamp.cmp(&a.timestamp)));
    findings
}

fn detect_current_user() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("LOGNAME"))
        .unwrap_or_else(|_| "unknown".to_string())
}

fn classify_user(user: &str) -> &'static str {
    match user {
        "root" | "daemon" | "bin" | "sys" | "sync" | "games" | "man" | "lp" | "mail" | "news"
        | "uucp" | "proxy" | "www-data" | "backup" | "list" | "irc" | "gnats" | "nobody" => {
            "service"
        }
        _ => "human",
    }
}

fn load_ids_config() -> IdsConfig {
    let path = Path::new("ids_config.json");
    let data = match fs::read_to_string(path) {
        Ok(v) => v,
        Err(_) => return IdsConfig::default(),
    };
    serde_json::from_str(&data).unwrap_or_default()
}

fn is_allowlisted(raw_lower: &str, config: &IdsConfig) -> bool {
    config
        .allowlist_commands
        .iter()
        .any(|rule| raw_lower.contains(&rule.to_lowercase()))
}

fn apply_suppression(matches: Vec<RuleMatch>, config: &IdsConfig) -> Vec<RuleMatch> {
    matches
        .into_iter()
        .filter(|m| !config.suppress_rules.iter().any(|id| id == m.id))
        .collect()
}

fn classify_tags(raw_lower: &str) -> Vec<CmdTag> {
    let mut tags = Vec::new();

    if contains_any(
        raw_lower,
        &["nmap", "masscan", "netstat", "ss ", "lsof", "whoami", "id "],
    ) {
        tags.push(CmdTag::Recon);
    }
    if contains_any(raw_lower, &["curl ", "wget ", "ftp ", "tftp "]) {
        tags.push(CmdTag::Download);
    }
    if contains_any(
        raw_lower,
        &["bash", "sh ", "zsh", "python", "perl", "ruby", "node"],
    ) {
        tags.push(CmdTag::Exec);
    }
    if contains_any(raw_lower, &["sudo ", "su ", "doas "]) {
        tags.push(CmdTag::PrivEsc);
    }

    tags
}

fn sequence_recon_download_exec(window: &VecDeque<(i64, Vec<CmdTag>)>) -> bool {
    let mut has_recon = false;
    let mut has_download = false;

    for (_, tags) in window {
        if !has_recon && tags.contains(&CmdTag::Recon) {
            has_recon = true;
            continue;
        }
        if has_recon && !has_download && tags.contains(&CmdTag::Download) {
            has_download = true;
            continue;
        }
        if has_recon && has_download && tags.contains(&CmdTag::Exec) {
            return true;
        }
    }
    false
}

fn sequence_sudo_shell(window: &VecDeque<(i64, Vec<CmdTag>)>) -> bool {
    let mut has_sudo = false;
    for (_, tags) in window {
        if !has_sudo && tags.contains(&CmdTag::PrivEsc) {
            has_sudo = true;
            continue;
        }
        if has_sudo && tags.contains(&CmdTag::Exec) {
            return true;
        }
    }
    false
}

fn rule_high_risk(raw: &str) -> Vec<RuleMatch> {
    let mut out = Vec::new();
    if raw.contains("|") && contains_any(raw, &["| sh", "| bash", "|bash", "| zsh", "|zsh"]) {
        out.push(RuleMatch {
            id: "highrisk.pipe_shell",
            score: 10,
            reason: "pipe to shell",
        });
    }
    if contains_any(raw, &["curl ", "wget "]) && contains_any(raw, &["| sh", "| bash", "|bash"]) {
        out.push(RuleMatch {
            id: "highrisk.remote_exec",
            score: 12,
            reason: "download and execute",
        });
    }
    out
}

fn rule_lolbins(raw: &str) -> Vec<RuleMatch> {
    let mut out = Vec::new();
    if contains_any(
        raw,
        &[
            "python -c",
            "perl -e",
            "ruby -e",
            "awk ",
            "nc -e",
            "bash -i",
        ],
    ) {
        out.push(RuleMatch {
            id: "lolbin.inline_exec",
            score: 6,
            reason: "lolbin inline execution",
        });
    }
    out
}

fn rule_history_tamper(raw: &str) -> Vec<RuleMatch> {
    let mut out = Vec::new();
    if contains_any(
        raw,
        &[
            "history -c",
            "unset histfile",
            "histfile=/dev/null",
            "rm ~/.bash_history",
            "rm ~/.zsh_history",
        ],
    ) {
        out.push(RuleMatch {
            id: "tamper.history",
            score: 8,
            reason: "history tampering",
        });
    }
    out
}

fn rule_persistence(raw: &str) -> Vec<RuleMatch> {
    let mut out = Vec::new();
    if contains_any(
        raw,
        &[
            "crontab",
            "systemctl enable",
            "launchctl",
            "/etc/rc.local",
            "~/.bashrc",
            "~/.zshrc",
            "~/.profile",
            "~/.config/autostart",
        ],
    ) {
        out.push(RuleMatch {
            id: "persist.setup",
            score: 7,
            reason: "persistence modification",
        });
    }
    out
}

fn rule_recon(raw: &str) -> Vec<RuleMatch> {
    let mut out = Vec::new();
    if contains_any(
        raw,
        &[
            "nmap", "masscan", "netstat", "ss ", "lsof", "whoami", "id ", "uname", "ifconfig",
            "ip a", "ip addr", "ps aux", "ps -ef", "last", "w ",
        ],
    ) {
        out.push(RuleMatch {
            id: "recon.basic",
            score: 3,
            reason: "recon command",
        });
    }
    out
}

fn rule_priv_esc(raw: &str) -> Vec<RuleMatch> {
    let mut out = Vec::new();
    if contains_any(raw, &["sudo ", "su ", "doas "]) {
        out.push(RuleMatch {
            id: "priv.esc",
            score: 3,
            reason: "privilege escalation",
        });
    }
    if raw.starts_with("sudo ") && contains_any(raw, &["bash", "sh ", "zsh", "python", "perl"]) {
        out.push(RuleMatch {
            id: "priv.shell",
            score: 5,
            reason: "privileged shell",
        });
    }
    out
}

fn rule_remote_exec(raw: &str) -> Vec<RuleMatch> {
    let mut out = Vec::new();
    if contains_any(raw, &["bash -c", "sh -c"]) && contains_any(raw, &["$(curl", "$(wget"]) {
        out.push(RuleMatch {
            id: "remote.exec.subshell",
            score: 10,
            reason: "subshell download exec",
        });
    }
    out
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| haystack.contains(n))
}

#[allow(dead_code)]
fn status_to_score(status: &CommandStatus) -> i32 {
    match status {
        CommandStatus::Success => 0,
        CommandStatus::Unknown => 1,
        CommandStatus::Error(_) => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::status_to_score;
    use crate::storage::db::CommandStatus;

    #[test]
    fn status_to_score_maps_statuses() {
        assert_eq!(status_to_score(&CommandStatus::Success), 0);
        assert_eq!(status_to_score(&CommandStatus::Unknown), 1);
        assert_eq!(status_to_score(&CommandStatus::Error(42)), 2);
    }
}
