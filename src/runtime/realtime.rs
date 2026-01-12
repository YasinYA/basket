use notify::{EventKind, RecursiveMode, Result, Watcher};
use serde::Deserialize;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use std::time::Duration;

use crate::storage::history::save_history_realtime;
use crate::util::logging::{log_to_console, Status};

/// Internal poller shared by polling & watcher
#[warn(dead_code)]
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

fn process_notify_stream<I>(events: I, path: &str, out_tx: &std::sync::mpsc::Sender<WatchEvent>)
where
    I: IntoIterator<Item = notify::Result<notify::Event>>,
{
    let mut last_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

    for res in events {
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
                if let Ok(mut file) = File::open(path) {
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
}
fn handle_realtime_line(line: &str) {
    handle_realtime_line_with(line, save_history_realtime);
}

fn handle_realtime_line_with<F>(line: &str, save_fn: F)
where
    F: FnOnce(&str, i32, i64) -> std::result::Result<(), Box<dyn std::error::Error>>,
{
    let parsed: RealtimeLine = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(e) => {
            log_to_console(
                &format!("Invalid realtime JSON: {} | {}", e, line),
                Status::WARNING,
            );
            return;
        }
    };

    if let Err(err) = save_fn(&parsed.cmd, parsed.status, parsed.timestamp) {
        log_to_console(
            &format!("Failed to save realtime command: {}", err),
            Status::ERROR,
        );
    }
}

fn process_watch_events(rx: Receiver<WatchEvent>, max_events: Option<usize>) {
    let mut handled = 0usize;
    for event in rx {
        match event {
            WatchEvent::Line(line) => {
                handle_realtime_line(&line);
            }
            WatchEvent::Error(err) => {
                log_to_console(&format!("Realtime watcher error: {}", err), Status::ERROR);
            }
        }

        handled += 1;
        if let Some(max) = max_events {
            if handled >= max {
                break;
            }
        }
    }
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

        process_notify_stream(rx, &path, &out_tx);
    });

    Ok(out_rx)
}

fn save_commands_on_poll_with(path: &str, max_loops: Option<usize>) {
    let mut poller = match FilePoller::new(path) {
        Ok(poller) => poller,
        Err(err) => {
            log_to_console(
                &format!("Realtime poller init failed: {}", err),
                Status::ERROR,
            );
            return;
        }
    };

    let mut loops = 0usize;
    loop {
        poll_once(&mut poller);
        loops += 1;
        if let Some(max) = max_loops {
            if loops >= max {
                break;
            }
        }
        thread::sleep(Duration::from_millis(500));
    }
}

fn save_commands_on_poll() {
    let path = format!(
        "{}/Documents/playground/basket/.cmdlog.json",
        std::env::var("HOME").expect("HOME not set")
    );
    save_commands_on_poll_with(&path, None);
}

fn poll_once(poller: &mut FilePoller) {
    match poller.poll() {
        Ok(lines) => {
            for line in lines {
                handle_realtime_line(&line);
            }
        }
        Err(err) => {
            log_to_console(&format!("Realtime poll error: {}", err), Status::ERROR);
        }
    }
}

fn save_realtime_commands_with<W, P>(watcher: W, poller: P, max_events: Option<usize>) -> Result<()>
where
    W: FnOnce() -> Result<Receiver<WatchEvent>>,
    P: FnOnce(),
{
    let rx = match watcher() {
        Ok(rx) => rx,
        Err(err) => {
            log_to_console(
                &format!("Realtime watcher failed, falling back to polling: {}", err),
                Status::WARNING,
            );
            poller();
            return Ok(());
        }
    };

    process_watch_events(rx, max_events);

    Ok(())
}

pub fn save_realtime_commands() -> Result<()> {
    save_realtime_commands_with(watch_cmdlog, save_commands_on_poll, None)
}

pub mod testing {
    use super::{
        handle_realtime_line_with, poll_once, process_notify_stream, FilePoller, WatchEvent,
    };
    use std::sync::mpsc::{channel, Receiver, Sender};

    pub struct Poller {
        inner: FilePoller,
    }

    impl Poller {
        pub fn new(path: &str) -> std::io::Result<Self> {
            Ok(Self {
                inner: FilePoller::new(path)?,
            })
        }

        pub fn poll(&mut self) -> std::io::Result<Vec<String>> {
            self.inner.poll()
        }

        pub fn poll_once(&mut self) {
            poll_once(&mut self.inner);
        }
    }

    pub fn handle_line_with<F>(line: &str, save_fn: F)
    where
        F: FnOnce(&str, i32, i64) -> std::result::Result<(), Box<dyn std::error::Error>>,
    {
        handle_realtime_line_with(line, save_fn);
    }

    pub fn channel_with_events(events: Vec<WatchEvent>) -> Receiver<WatchEvent> {
        let (tx, rx) = channel();
        for event in events {
            let _ = tx.send(event);
        }
        rx
    }

    pub fn run_with_events(rx: Receiver<WatchEvent>, max_events: Option<usize>) {
        super::process_watch_events(rx, max_events);
    }

    pub fn save_realtime_with<W, P>(
        watcher: W,
        poller: P,
        max_events: Option<usize>,
    ) -> notify::Result<()>
    where
        W: FnOnce() -> notify::Result<Receiver<WatchEvent>>,
        P: FnOnce(),
    {
        super::save_realtime_commands_with(watcher, poller, max_events)
    }

    pub fn run_notify_events<I>(events: I, path: &str, out_tx: &Sender<WatchEvent>)
    where
        I: IntoIterator<Item = notify::Result<notify::Event>>,
    {
        process_notify_stream(events, path, out_tx);
    }

    pub fn run_polling_once(path: &str) {
        super::save_commands_on_poll_with(path, Some(1));
    }
}
