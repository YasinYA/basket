#[cfg(not(test))]
use basket::app::{default_deps, run_app};

#[cfg(not(test))]
fn main() {
    run_main(|| run_app(&default_deps()));
}

fn run_main<F: FnOnce()>(run: F) {
    println!(
        r#"
    _               _        _
   | |             | |      | |
   | |__   __ _ ___| | _____| |_
   | '_ \ / _` / __| |/ / _ \ __|
   | |_) | (_| \__ \   <  __/ |_
   |_.__/ \__,_|___/_|\_\___|\__|
   S E E  Y O U   L A T E R ! ! !
       "#
    );
    run();
}

#[cfg(test)]
mod tests {
    use super::run_main;
    use std::cell::Cell;

    #[test]
    fn run_main_invokes_runner() {
        let called = Cell::new(false);
        run_main(|| called.set(true));
        assert!(called.get());
    }
}
