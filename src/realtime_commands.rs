use notify::{EventKind, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::duration;

fn watch_cmdlog(path: &str) -> notify::Result<()> {
    let (tx, rx) = channel();
    watcher = notify::recommended_watcher(tx);
    watcher.watch(Path::new(path), RecursiveMode::NonRecursive);
}
