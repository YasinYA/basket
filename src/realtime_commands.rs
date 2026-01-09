use notify::event::{DataChange, ModifyKind};
use notify::{EventKind, RecursiveMode, Result, Watcher};
use serde::Deserialize;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::mpsc::{channel, Receiver};
use std::thread;

use crate::history_file::save_history_realtime;
use crate::logging::{log_to_console, Status};

/// Internal poller shared by polling & watcher
struct FilePoller {
    path: String,
    last_size: u64,
}

#[derive(Debug, Deserialize)]
pub struct RealtimeLine {
    cmd: String,
    status: i32,
    timestamp: i64,
}

pub enum WatchEvent {
    Line(String),
    Error(std::io::Error),
}

impl FilePoller {
    fn new(path: &str) -> std::io::Result<Self> {
        let last_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        Ok(Self {
            path: path.to_string(),
            last_size,
        })
    }

    fn poll(&mut self) -> std::io::Result<Vec<String>> {
        let mut file = File::open(&self.path)?;
        let new_size = file.metadata()?.len();

        // File replaced or truncated
        if new_size < self.last_size {
            self.last_size = 0;
        }

        if new_size == self.last_size {
            return Ok(vec![]);
        }

        file.seek(SeekFrom::Start(self.last_size))?;
        let mut buf = String::new();
        file.read_to_string(&mut buf)?;

        self.last_size = new_size;

        Ok(buf.lines().map(|l| l.to_string()).collect())
    }
}

pub fn watch_cmdlog() -> Result<Receiver<WatchEvent>> {
    let (out_tx, out_rx) = channel();

    let path = format!(
        "{}/Documents/playground/basket/.cmdlog.json",
        std::env::var("HOME").expect("HOME not set")
    );

    thread::spawn(move || {
        let (tx, rx) = channel();

        // 🔴 WATCHER MUST LIVE INSIDE THE THREAD
        let mut watcher = notify::recommended_watcher(tx).expect("failed to create watcher");

        watcher
            .watch(Path::new(&path), RecursiveMode::NonRecursive)
            .expect("failed to watch file");

        let mut last_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

        for res in rx {
            let event = match res {
                Ok(e) => e,
                Err(e) => {
                    let _ = out_tx.send(WatchEvent::Error(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e,
                    )));
                    continue;
                }
            };

            match event.kind {
                EventKind::Modify(_) | EventKind::Create(_) => {
                    if let Ok(mut file) = File::open(&path) {
                        let new_size = file.metadata().map(|m| m.len()).unwrap_or(0);

                        if new_size < last_size {
                            last_size = 0;
                        }

                        if new_size > last_size {
                            file.seek(SeekFrom::Start(last_size)).ok();
                            let mut buf = String::new();
                            file.read_to_string(&mut buf).ok();

                            for line in buf.lines() {
                                let _ = out_tx.send(WatchEvent::Line(line.to_string()));
                            }

                            last_size = new_size;
                        }
                    }
                }
                _ => {}
            }
        }
    });

    Ok(out_rx)
}

pub fn save_realtime_commands() -> Result<()> {
    // Fails only if watcher cannot start
    let rx = watch_cmdlog()?;

    for event in rx {
        match event {
            WatchEvent::Line(line) => {
                // Parse JSON line
                let parsed: RealtimeLine = match serde_json::from_str(&line) {
                    Ok(v) => v,
                    Err(e) => {
                        log_to_console(
                            &format!("Invalid realtime JSON: {} | {}", e, line),
                            Status::WARNING,
                        );
                        continue;
                    }
                };

                // Save to DB
                if let Err(err) =
                    save_history_realtime(&parsed.cmd, parsed.status, parsed.timestamp)
                {
                    log_to_console(
                        &format!("Failed to save realtime command: {}", err),
                        Status::ERROR,
                    );
                }
            }

            WatchEvent::Error(err) => {
                log_to_console(&format!("Realtime watcher error: {}", err), Status::ERROR);
            }
        }
    }

    Ok(())
}
