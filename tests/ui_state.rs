use basket::ui::testing::TestState;
use crossterm::event::KeyCode;
use std::time::Duration;

#[test]
fn handle_key_events_cover_menu_and_overview() {
    let menu_items = ["View overview", "Start realtime watcher", "Exit"];
    let mut state = TestState::new();

    assert_eq!(state.menu_index(), 0);
    state.handle_key(KeyCode::Down, &menu_items);
    assert_eq!(state.menu_index(), 1);
    state.handle_key(KeyCode::Up, &menu_items);
    assert_eq!(state.menu_index(), 0);
    state.handle_key(KeyCode::Up, &menu_items);
    assert_eq!(state.menu_index(), menu_items.len() - 1);
    state.handle_key(KeyCode::Down, &menu_items);
    assert_eq!(state.menu_index(), 0);

    state.handle_key(KeyCode::Enter, &menu_items);
    assert_eq!(state.screen(), "overview");

    state.handle_key(KeyCode::Char('c'), &menu_items);
    assert_eq!(
        state.overview_view(),
        basket::ui::testing::OverviewView::Charts
    );
    state.handle_key(KeyCode::Char('t'), &menu_items);
    assert_eq!(
        state.overview_view(),
        basket::ui::testing::OverviewView::Tables
    );

    state.handle_key(KeyCode::Char('s'), &menu_items);
    state.handle_key(KeyCode::Char('/'), &menu_items);
    assert!(state.filter_input());

    let before = state.filter_buffer().to_string();
    state.handle_key(KeyCode::Char('\u{7f}'), &menu_items);
    assert_eq!(state.filter_buffer(), before);

    state.handle_key(KeyCode::Esc, &menu_items);
    assert!(!state.filter_input());

    state.handle_key(KeyCode::Char('a'), &menu_items);
    state.handle_key(KeyCode::Backspace, &menu_items);
    state.handle_key(KeyCode::Enter, &menu_items);
    assert!(!state.filter_input());

    state.handle_key(KeyCode::Left, &menu_items);
    state.handle_key(KeyCode::Right, &menu_items);
    state.handle_key(KeyCode::Up, &menu_items);
    state.handle_key(KeyCode::Down, &menu_items);

    state.handle_key(KeyCode::Char('b'), &menu_items);
    assert_eq!(state.screen(), "menu");

    state.handle_key(KeyCode::Down, &menu_items);
    state.handle_key(KeyCode::Enter, &menu_items);
    assert_eq!(state.screen(), "watcher");

    let mut already_started = TestState::new();
    already_started.set_menu_index(1);
    already_started.set_watcher_started(true);
    already_started.handle_key(KeyCode::Enter, &menu_items);
    assert_eq!(already_started.screen(), "watcher");

    let mut error_state = TestState::new();
    error_state.handle_key(KeyCode::Enter, &menu_items);
    assert_eq!(error_state.screen(), "overview");

    let mut error_refresh = TestState::new();
    error_refresh.handle_key_with_error(KeyCode::Enter, &menu_items);
    assert_eq!(error_refresh.screen(), "error");
}

#[test]
fn handle_key_events_cover_error_and_exit_paths() {
    let menu_items = ["View overview", "Start realtime watcher", "Exit"];
    let mut state = TestState::new();

    let exit = state.handle_key(KeyCode::Char('q'), &menu_items);
    assert!(exit.is_some());

    state.set_screen_overview();
    let exit = state.handle_key(KeyCode::Char('q'), &menu_items);
    assert!(exit.is_some());

    state.set_screen_watcher();
    let exit = state.handle_key(KeyCode::Char('q'), &menu_items);
    assert!(exit.is_some());

    state.set_screen_error("boom");
    let exit = state.handle_key(KeyCode::Char('q'), &menu_items);
    assert!(exit.is_some());
}

#[test]
fn tick_app_updates_animated_state() {
    let mut state = TestState::new();
    let tick_rate = Duration::from_millis(1);
    let refresh_rate = Duration::from_millis(1);

    state.set_screen_overview();
    state.set_overview_view(basket::ui::testing::OverviewView::Charts);
    state.set_watcher_started(true);
    state.set_last_refresh_offset(Duration::from_millis(10));
    state.set_last_tick_offset(Duration::from_millis(10));
    state.tick(tick_rate, refresh_rate).expect("tick");
    assert!(state.chart_progress() > 0.0);

    state.set_screen_watcher();
    let before = state.watcher_spinner();
    state.set_last_tick_offset(Duration::from_millis(10));
    state.tick(tick_rate, refresh_rate).expect("tick");
    assert!(state.watcher_spinner() != before);
}
