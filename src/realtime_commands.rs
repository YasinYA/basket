use notify::{RecursiveMode, Result, Watcher};
use std::path::Path;
use std::sync::mpsc::channel;

fn watch_path(path: &str) -> Result<()> {
    let (tx, rx) = channel();
    let mut watcher = notify::recommended_watcher(tx)?;
    let _ = watcher.watch(Path::new(path), RecursiveMode::NonRecursive)?;

    for res in rx {
        match res {
            Ok(event) => println!("event: {:?}", event),
            Err(err) => println!("watch error: {:?}", err),
        }
    }

    Ok(())
}

pub fn watch_cmdlog() {
    let _ = watch_path(".cmdlog.json");
}
