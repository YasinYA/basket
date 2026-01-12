mod common;

use basket::runtime::realtime::testing::{
    channel_with_events, handle_line_with, run_notify_events, run_polling_once, run_with_events,
    save_realtime_with, Poller,
};
use basket::runtime::realtime::{watch_cmdlog, WatchEvent};
use notify::event::ModifyKind;
use notify::EventKind;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

fn temp_file(name: &str) -> std::path::PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("basket_realtime_{}", nonce));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir.join(name)
}

#[test]
fn file_poller_reads_appended_lines_and_truncation() {
    let path = temp_file("cmdlog.json");
    fs::File::create(&path).expect("create file");

    let mut poller = Poller::new(path.to_str().expect("path")).expect("poller");
    {
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("open append");
        writeln!(file, "one").expect("write line");
    }

    let first = poller.poll().expect("poll first");
    assert_eq!(first, vec!["one".to_string()]);

    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .expect("open append");
    writeln!(file, "two").expect("append");

    let second = poller.poll().expect("poll second");
    assert_eq!(second, vec!["two".to_string()]);

    fs::write(&path, "short\n").expect("truncate file");
    let third = poller.poll().expect("poll after truncation");
    assert_eq!(third, vec!["short".to_string()]);

    poller.poll_once();
}

#[test]
fn handle_realtime_line_calls_save_for_valid_json() {
    let called = std::cell::Cell::new(false);
    let line = r#"{"cmd":"ls","status":0,"timestamp":123}"#;
    handle_line_with(line, |cmd, status, ts| {
        called.set(true);
        assert_eq!(cmd, "ls");
        assert_eq!(status, 0);
        assert_eq!(ts, 123);
        Ok(())
    });
    assert!(called.get());
}

#[test]
fn handle_realtime_line_ignores_invalid_json() {
    let called = std::cell::Cell::new(false);
    handle_line_with("not json", |_cmd, _status, _ts| {
        called.set(true);
        Ok(())
    });
    assert!(!called.get());
}

#[test]
fn handle_realtime_line_logs_on_save_error() {
    let called = std::cell::Cell::new(false);
    let line = r#"{"cmd":"ls","status":0,"timestamp":123}"#;
    handle_line_with(line, |_cmd, _status, _ts| {
        called.set(true);
        Err("save failed".into())
    });
    assert!(called.get());
}

#[test]
fn process_watch_events_handles_line_and_error() {
    let events = vec![
        WatchEvent::Line(r#"{"cmd":"ls","status":0,"timestamp":1}"#.to_string()),
        WatchEvent::Error(std::io::Error::new(std::io::ErrorKind::Other, "boom")),
    ];
    let rx = channel_with_events(events);
    run_with_events(rx, Some(2));
}

#[test]
fn save_realtime_commands_fallback_calls_poller() {
    let called = std::cell::Cell::new(false);
    let result = save_realtime_with(
        || Err(notify::Error::generic("watcher fail")),
        || {
            called.set(true);
        },
        Some(1),
    );
    assert!(result.is_ok());
    assert!(called.get());
}

#[test]
fn save_realtime_commands_processes_events() {
    let (tx, rx) = std::sync::mpsc::channel();
    tx.send(WatchEvent::Line(
        r#"{"cmd":"ls","status":0,"timestamp":1}"#.to_string(),
    ))
    .expect("send line");
    drop(tx);

    let result = save_realtime_with(|| Ok(rx), || {}, Some(1));
    assert!(result.is_ok());
}

#[test]
fn process_notify_stream_reads_lines_and_errors() {
    let path = temp_file("notify.json");
    std::fs::File::create(&path).expect("create file");
    let path_buf = PathBuf::from(&path);
    let mut step = 0;

    let (tx, rx) = std::sync::mpsc::channel();
    let events = std::iter::from_fn(move || {
        if step == 0 {
            step += 1;
            std::fs::write(&path_buf, "one\n").ok();
            return Some(Ok(notify::Event {
                kind: EventKind::Modify(ModifyKind::Any),
                paths: vec![path_buf.clone()],
                attrs: Default::default(),
            }));
        }
        if step == 1 {
            step += 1;
            return Some(Err(notify::Error::generic("boom")));
        }
        None
    });

    run_notify_events(events, path.to_str().expect("path str"), &tx);
    drop(tx);

    let events: Vec<WatchEvent> = rx.try_iter().collect();
    assert!(events
        .iter()
        .any(|event| matches!(event, WatchEvent::Line(_))));
    assert!(events
        .iter()
        .any(|event| matches!(event, WatchEvent::Error(_))));
}

#[test]
fn process_notify_stream_ignores_unrelated_events() {
    let path = temp_file("other.json");
    std::fs::File::create(&path).expect("create file");
    let event = notify::Event {
        kind: EventKind::Other,
        paths: vec![path.clone()],
        attrs: Default::default(),
    };

    let (tx, rx) = std::sync::mpsc::channel();
    run_notify_events([Ok(event)], path.to_str().expect("path str"), &tx);
    drop(tx);
    assert!(rx.try_iter().next().is_none());
}

#[test]
fn watch_cmdlog_initializes_watcher() {
    let _guard = common::env_lock();
    let home = std::env::temp_dir().join("basket_watch_init");
    let _ = fs::remove_dir_all(&home);
    let cmdlog_dir = home.join("Documents/playground/basket");
    fs::create_dir_all(&cmdlog_dir).expect("create cmdlog dir");
    let cmdlog_path = cmdlog_dir.join(".cmdlog.json");
    fs::File::create(&cmdlog_path).expect("create cmdlog file");

    let prev_home = std::env::var("HOME").ok();
    std::env::set_var("HOME", &home);

    let rx = watch_cmdlog().expect("watcher");
    drop(rx);

    match prev_home {
        Some(val) => std::env::set_var("HOME", val),
        None => std::env::remove_var("HOME"),
    }
}

#[test]
fn polling_loop_runs_once() {
    let path = temp_file("poll_once.json");
    fs::File::create(&path).expect("create file");
    run_polling_once(path.to_str().expect("path str"));
}

#[test]
fn polling_loop_handles_missing_file() {
    let path = temp_file("missing.json");
    run_polling_once(path.to_str().expect("path str"));
}

#[test]
fn poll_once_handles_missing_file_after_init() {
    let path = temp_file("missing_after.json");
    fs::File::create(&path).expect("create file");
    let mut poller = Poller::new(path.to_str().expect("path")).expect("poller");
    fs::remove_file(&path).expect("remove file");
    poller.poll_once();
}

#[test]
fn notify_stream_handles_truncation() {
    let path = temp_file("truncate.json");
    fs::write(&path, "one\n").expect("write file");
    let path_buf = PathBuf::from(&path);
    let mut step = 0;
    let events = std::iter::from_fn(move || {
        if step == 0 {
            step += 1;
            fs::write(&path_buf, "").ok();
            return Some(Ok(notify::Event {
                kind: EventKind::Modify(ModifyKind::Any),
                paths: vec![path_buf.clone()],
                attrs: Default::default(),
            }));
        }
        None
    });

    let (tx, rx) = std::sync::mpsc::channel();
    run_notify_events(events, path.to_str().expect("path str"), &tx);
    drop(tx);
    assert!(rx.try_iter().next().is_none());
}
