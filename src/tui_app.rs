use std::error::Error;
use std::io;
use std::thread;
use std::time::{Duration, Instant};

use chrono::Utc;
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::prelude::Frame;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table, TableState};
use ratatui::Terminal;
use tui_piechart::{symbols, PieChart, PieSlice, Resolution};

use crate::analyze_commands::{get_overview_tables, OverviewTables};
use crate::db::{get_recent_entries, CommandStatus, Entry};
use crate::ids::{detect_intrusions, IntrusionFinding};
use crate::realtime_commands::save_realtime_commands;

struct Theme {
    gold: Color,
    gold_bright: Color,
    menu_inactive: Color,
    menu_active: Color,
    menu_border: Color,
    footer_border: Color,
    header_text: Color,
    table_border: Color,
    table_header: Color,
    table_row_even: Color,
    table_row_odd: Color,
    table_active_bg: Color,
    table_active_fg: Color,
    logo_colors: [Color; 6],
    chart_palette: [Color; 6],
    legend_text: Color,
    legend_dim: Color,
}

const THEME: Theme = Theme {
    gold: Color::Rgb(148, 92, 255),
    gold_bright: Color::Rgb(200, 160, 255),
    menu_inactive: Color::White,
    menu_active: Color::Rgb(148, 92, 255),
    menu_border: Color::Rgb(148, 92, 255),
    footer_border: Color::Rgb(148, 92, 255),
    header_text: Color::Rgb(186, 147, 255),
    table_border: Color::Rgb(148, 92, 255),
    table_header: Color::Rgb(190, 160, 255),
    table_row_even: Color::White,
    table_row_odd: Color::Gray,
    table_active_bg: Color::Rgb(148, 92, 255),
    table_active_fg: Color::White,
    logo_colors: [
        Color::Rgb(210, 185, 255),
        Color::Rgb(186, 147, 255),
        Color::Rgb(164, 110, 255),
        Color::Rgb(148, 92, 255),
        Color::Rgb(164, 110, 255),
        Color::Rgb(186, 147, 255),
    ],
    chart_palette: [
        Color::Rgb(148, 92, 255),
        Color::Rgb(98, 76, 255),
        Color::Rgb(186, 147, 255),
        Color::Rgb(230, 180, 255),
        Color::Rgb(122, 102, 255),
        Color::Rgb(200, 160, 255),
    ],
    legend_text: Color::Gray,
    legend_dim: Color::DarkGray,
};

pub enum TuiExit {
    Exit,
}

enum Screen {
    Menu,
    Overview,
    Watcher,
    Error(String),
}

#[derive(Copy, Clone)]
enum OverviewView {
    Tables,
    Charts,
}

struct AppState {
    screen: Screen,
    menu_index: usize,
    overview: Option<OverviewTables>,
    logo_phase: usize,
    last_tick: Instant,
    overview_view: OverviewView,
    chart_progress: f32,
    chart_index: usize,
    chart_selection: [usize; 3],
    table_index: usize,
    table_selection: [usize; 3],
    sort_asc: bool,
    filter_query: String,
    filter_input: bool,
    filter_buffer: String,
    watcher_started: bool,
    watcher_spinner: usize,
    recent_rows: Vec<Vec<String>>,
    intrusion_rows: Vec<Vec<String>>,
    last_refresh: Instant,
}

impl AppState {
    fn new() -> Self {
        Self {
            screen: Screen::Menu,
            menu_index: 0,
            overview: None,
            logo_phase: 0,
            last_tick: Instant::now(),
            overview_view: OverviewView::Tables,
            chart_progress: 0.0,
            chart_index: 0,
            chart_selection: [0, 0, 0],
            table_index: 0,
            table_selection: [0, 0, 0],
            sort_asc: true,
            filter_query: String::new(),
            filter_input: false,
            filter_buffer: String::new(),
            watcher_started: false,
            watcher_spinner: 0,
            recent_rows: Vec::new(),
            intrusion_rows: Vec::new(),
            last_refresh: Instant::now(),
        }
    }
}

pub fn run() -> Result<TuiExit, Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_loop(&mut terminal);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<TuiExit, Box<dyn Error>> {
    let menu_items = ["View overview", "Start realtime watcher", "Exit"];
    let mut app = AppState::new();
    let exit_action = TuiExit::Exit;

    let tick_rate = Duration::from_millis(120);
    let refresh_rate = Duration::from_secs(1);

    loop {
        if app.last_tick.elapsed() >= tick_rate {
            app.logo_phase = (app.logo_phase + 1) % 6;
            if matches!(app.screen, Screen::Overview)
                && matches!(app.overview_view, OverviewView::Charts)
                && app.chart_progress < 1.0
            {
                app.chart_progress = (app.chart_progress + 0.08).min(1.0);
            }
            if matches!(app.screen, Screen::Overview)
                && app.watcher_started
                && app.last_refresh.elapsed() >= refresh_rate
            {
                if let Err(err) = refresh_overview_state(&mut app) {
                    app.screen = Screen::Error(err);
                }
            }
            if matches!(app.screen, Screen::Watcher) {
                app.watcher_spinner = (app.watcher_spinner + 1) % spinner_frames().len();
            }
            app.last_tick = Instant::now();
        }

        terminal.draw(|frame| match &app.screen {
            Screen::Menu => render_menu(frame, &menu_items, app.menu_index, app.logo_phase),
            Screen::Overview => render_overview(
                frame,
                app.overview.as_ref(),
                app.overview_view,
                app.chart_progress,
                app.chart_index,
                app.chart_selection,
                app.table_index,
                app.table_selection,
                &app.filter_query,
                app.sort_asc,
                app.filter_input,
                &app.filter_buffer,
                &app.recent_rows,
                &app.intrusion_rows,
            ),
            Screen::Watcher => render_watcher(frame, app.watcher_spinner),
            Screen::Error(message) => render_error(frame, message),
        })?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if app.filter_input {
                    match key.code {
                        KeyCode::Enter => {
                            app.filter_query = app.filter_buffer.trim().to_string();
                            app.filter_input = false;
                            app.table_selection = [0, 0, 0];
                        }
                        KeyCode::Esc => {
                            app.filter_query.clear();
                            app.filter_buffer.clear();
                            app.filter_input = false;
                            app.table_selection = [0, 0, 0];
                        }
                        KeyCode::Backspace => {
                            app.filter_buffer.pop();
                        }
                        KeyCode::Char(c) => {
                            if !c.is_control() {
                                app.filter_buffer.push(c);
                            }
                        }
                        _ => {}
                    }
                    continue;
                }
                match app.screen {
                    Screen::Menu => match key.code {
                        KeyCode::Up => {
                            if app.menu_index == 0 {
                                app.menu_index = menu_items.len() - 1;
                            } else {
                                app.menu_index -= 1;
                            }
                        }
                        KeyCode::Down => {
                            app.menu_index = (app.menu_index + 1) % menu_items.len();
                        }
                        KeyCode::Enter => match app.menu_index {
                            0 => {
                                if let Err(err) = refresh_overview_state(&mut app) {
                                    app.screen = Screen::Error(err);
                                } else {
                                    app.screen = Screen::Overview;
                                }
                            }
                            1 => {
                                app.screen = Screen::Watcher;
                                if !app.watcher_started {
                                    app.watcher_started = true;
                                    thread::spawn(|| {
                                        let _ = save_realtime_commands();
                                    });
                                }
                            }
                            _ => return Ok(exit_action),
                        },
                        KeyCode::Char('q') => return Ok(exit_action),
                        _ => {}
                    },
                    Screen::Overview => match key.code {
                        KeyCode::Char('c') => {
                            app.overview_view = OverviewView::Charts;
                            app.chart_progress = 0.0;
                            app.chart_index = 0;
                        }
                        KeyCode::Char('t') => {
                            app.overview_view = OverviewView::Tables;
                            app.table_index = 0;
                        }
                        KeyCode::Char('s') => {
                            if matches!(app.overview_view, OverviewView::Tables) {
                                app.sort_asc = !app.sort_asc;
                                app.table_selection = [0, 0, 0];
                            }
                        }
                        KeyCode::Char('/') => {
                            if matches!(app.overview_view, OverviewView::Tables) {
                                app.filter_input = true;
                                app.filter_buffer = app.filter_query.clone();
                            }
                        }
                        KeyCode::Char('x') => {
                            if matches!(app.overview_view, OverviewView::Tables) {
                                app.filter_query.clear();
                                app.filter_buffer.clear();
                                app.table_selection = [0, 0, 0];
                            }
                        }
                        KeyCode::Left => {
                            if matches!(app.overview_view, OverviewView::Charts) {
                                if app.chart_index == 0 {
                                    app.chart_index = 2;
                                } else {
                                    app.chart_index -= 1;
                                }
                            } else if matches!(app.overview_view, OverviewView::Tables) {
                                if app.table_index == 0 {
                                    app.table_index = 2;
                                } else {
                                    app.table_index -= 1;
                                }
                            }
                        }
                        KeyCode::Right => {
                            if matches!(app.overview_view, OverviewView::Charts) {
                                app.chart_index = (app.chart_index + 1) % 3;
                            } else if matches!(app.overview_view, OverviewView::Tables) {
                                app.table_index = (app.table_index + 1) % 3;
                            }
                        }
                        KeyCode::Up => {
                            if matches!(app.overview_view, OverviewView::Charts) {
                                let len = chart_len(app.overview.as_ref(), app.chart_index);
                                if len > 0 {
                                    let idx = &mut app.chart_selection[app.chart_index];
                                    if *idx == 0 {
                                        *idx = len - 1;
                                    } else {
                                        *idx -= 1;
                                    }
                                }
                            } else if matches!(app.overview_view, OverviewView::Tables) {
                                let len = table_len_filtered(
                                    app.overview.as_ref(),
                                    app.table_index,
                                    &app.filter_query,
                                );
                                if len > 0 {
                                    let idx = &mut app.table_selection[app.table_index];
                                    if *idx == 0 {
                                        *idx = len - 1;
                                    } else {
                                        *idx -= 1;
                                    }
                                }
                            }
                        }
                        KeyCode::Down => {
                            if matches!(app.overview_view, OverviewView::Charts) {
                                let len = chart_len(app.overview.as_ref(), app.chart_index);
                                if len > 0 {
                                    let idx = &mut app.chart_selection[app.chart_index];
                                    *idx = (*idx + 1) % len;
                                }
                            } else if matches!(app.overview_view, OverviewView::Tables) {
                                let len = table_len_filtered(
                                    app.overview.as_ref(),
                                    app.table_index,
                                    &app.filter_query,
                                );
                                if len > 0 {
                                    let idx = &mut app.table_selection[app.table_index];
                                    *idx = (*idx + 1) % len;
                                }
                            }
                        }
                        KeyCode::Char('b') => {
                            app.screen = Screen::Menu;
                        }
                        KeyCode::Char('q') => return Ok(exit_action),
                        _ => {}
                    },
                    Screen::Watcher => match key.code {
                        KeyCode::Char('b') => {
                            app.screen = Screen::Menu;
                        }
                        KeyCode::Char('q') => return Ok(exit_action),
                        _ => {}
                    },
                    Screen::Error(_) => match key.code {
                        KeyCode::Char('b') => {
                            app.screen = Screen::Menu;
                        }
                        KeyCode::Char('q') => return Ok(exit_action),
                        _ => {}
                    },
                }
            }
        }
    }
}

fn render_menu(frame: &mut Frame, items: &[&str], selected: usize, logo_phase: usize) {
    let size = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Length(10),
            Constraint::Min(3),
            Constraint::Length(3),
        ])
        .split(size);

    let powered_by = Paragraph::new("Get insightful view from what you type everyday")
        .style(
            Style::default()
                .fg(THEME.header_text)
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Basket")
                .border_style(Style::default().fg(THEME.gold)),
        );

    let logo = Paragraph::new(basket_logo_text(logo_phase)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(THEME.gold)),
    );

    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let prefix = if idx == selected { "> " } else { "  " };
            let item_style = if idx == selected {
                Style::default()
                    .fg(THEME.menu_active)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(THEME.menu_inactive)
            };
            ListItem::new(Line::from(vec![
                Span::styled(prefix, item_style),
                Span::styled(*item, item_style),
            ]))
        })
        .collect();

    let list = List::new(list_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Menu")
                .border_style(Style::default().fg(THEME.menu_border)),
        )
        .highlight_style(Style::default());

    let footer = Paragraph::new("Use Up/Down + Enter. Press q to quit.")
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(THEME.footer_border)),
        )
        .style(Style::default().fg(THEME.footer_border));

    frame.render_widget(powered_by, chunks[0]);
    frame.render_widget(logo, chunks[1]);
    frame.render_widget(list, chunks[2]);
    frame.render_widget(footer, chunks[3]);
}

fn render_overview(
    frame: &mut Frame,
    tables: Option<&OverviewTables>,
    view: OverviewView,
    chart_progress: f32,
    chart_index: usize,
    chart_selection: [usize; 3],
    table_index: usize,
    table_selection: [usize; 3],
    filter_query: &str,
    sort_asc: bool,
    filter_input: bool,
    filter_buffer: &str,
    recent_rows: &[Vec<String>],
    intrusion_rows: &[Vec<String>],
) {
    let size = frame.area();
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(10),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(3),
        ])
        .split(size);

    if let Some(tables) = tables {
        match view {
            OverviewView::Tables => {
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(33),
                        Constraint::Percentage(34),
                        Constraint::Percentage(33),
                    ])
                    .split(layout[0]);

                let top_commands = filter_sort_rows(&tables.top_commands, filter_query, sort_asc);
                let top_unsuccessful =
                    filter_sort_rows(&tables.top_unsuccessful, filter_query, sort_asc);
                let mistyped_commands =
                    filter_sort_rows(&tables.mistyped_commands, filter_query, sort_asc);

                render_table(
                    frame,
                    chunks[0],
                    "Command Occurrences",
                    &top_commands,
                    table_selection[0],
                    table_index == 0,
                );
                render_table(
                    frame,
                    chunks[1],
                    "Most Unsuccessful Commands",
                    &top_unsuccessful,
                    table_selection[1],
                    table_index == 1,
                );
                render_table(
                    frame,
                    chunks[2],
                    "Most Mistyped Commands",
                    &mistyped_commands,
                    table_selection[2],
                    table_index == 2,
                );
            }
            OverviewView::Charts => {
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(33),
                        Constraint::Percentage(34),
                        Constraint::Percentage(33),
                    ])
                    .split(layout[0]);

                render_chart(
                    frame,
                    chunks[0],
                    "Command Occurrences",
                    &tables.top_commands,
                    chart_progress,
                    chart_selection[0],
                    chart_index == 0,
                );
                render_chart(
                    frame,
                    chunks[1],
                    "Most Unsuccessful Commands",
                    &tables.top_unsuccessful,
                    chart_progress,
                    chart_selection[1],
                    chart_index == 1,
                );
                render_chart(
                    frame,
                    chunks[2],
                    "Most Mistyped Commands",
                    &tables.mistyped_commands,
                    chart_progress,
                    chart_selection[2],
                    chart_index == 2,
                );
            }
        }
    } else {
        let empty = Paragraph::new("No overview data.").block(
            Block::default()
                .borders(Borders::ALL)
                .title("Overview")
                .border_style(Style::default().fg(THEME.gold)),
        );
        frame.render_widget(empty, layout[0]);
    }

    render_recent_table(frame, layout[1], recent_rows);
    render_intrusion_table(frame, layout[2], intrusion_rows);

    let sort_label = if sort_asc { "A->Z" } else { "Z->A" };
    let filter_label = if filter_query.is_empty() {
        "none"
    } else {
        filter_query
    };
    let footer = match view {
        OverviewView::Tables => Paragraph::new(format!(
            "t: tables  c: charts  ←/→: table  ↑/↓: row  s: sort({})  /: filter({})  x: clear  b: back  q: quit",
            sort_label, filter_label
        )),
        OverviewView::Charts => Paragraph::new(
            "t: tables  c: charts  ←/→: chart  ↑/↓: slice  b: back  q: quit",
        ),
    }
    .block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(THEME.footer_border)),
    );
    frame.render_widget(footer, layout[3]);

    if filter_input {
        render_filter_prompt(frame, layout[0], filter_buffer);
    }
}

fn render_error(frame: &mut Frame, message: &str) {
    let size = frame.area();
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Error")
        .border_style(Style::default().fg(THEME.gold));
    let paragraph = Paragraph::new(message)
        .block(block)
        .style(Style::default().fg(Color::Red));
    frame.render_widget(paragraph, size);
}

fn render_watcher(frame: &mut Frame, spinner_index: usize) {
    let size = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(size);

    let spinner = spinner_frames()[spinner_index];
    let content = format!(
        "{}  Realtime watcher is running\nListening for new commands...",
        spinner
    );

    let body = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Watcher")
                .border_style(Style::default().fg(THEME.gold)),
        )
        .style(
            Style::default()
                .fg(THEME.menu_inactive)
                .add_modifier(Modifier::BOLD),
        );

    let footer = Paragraph::new("b: back  q: quit")
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(THEME.footer_border)),
        )
        .style(Style::default().fg(THEME.footer_border));

    frame.render_widget(body, chunks[0]);
    frame.render_widget(footer, chunks[1]);
}

fn spinner_frames() -> [&'static str; 10] {
    ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]
}

fn render_table(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    data: &[Vec<String>],
    selected_index: usize,
    is_focused: bool,
) {
    let selected = selected_index.min(data.len().saturating_sub(1));
    let rows: Vec<Row> = if data.is_empty() {
        vec![Row::new(vec!["(no data)".to_string(), "".to_string()])]
    } else {
        let inner_width = area.width.saturating_sub(2) as usize;
        let col1_width = ((inner_width as f32) * 0.7).floor() as usize;
        let col2_width = inner_width.saturating_sub(col1_width + 1).max(1);

        let separator = Row::new(vec![
            "─".repeat(col1_width.max(1)),
            "─".repeat(col2_width.max(1)),
        ])
        .style(Style::default().fg(THEME.table_border));

        let mut out = Vec::with_capacity(data.len() + 1);
        out.push(separator);

        for (idx, row) in data.iter().enumerate() {
            let left = row.get(0).cloned().unwrap_or_default();
            let right = row.get(1).cloned().unwrap_or_default();
            let style = if idx % 2 == 0 {
                Style::default()
                    .fg(THEME.table_row_even)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(THEME.table_row_odd)
                    .add_modifier(Modifier::BOLD)
            };
            if is_focused && idx == selected {
                let left_text = Text::from(vec![
                    Line::raw(""),
                    Line::raw(format!("➤ {}", left)),
                    Line::raw(""),
                ]);
                let right_text = Text::from(vec![Line::raw(""), Line::raw(right), Line::raw("")]);
                out.push(Row::new(vec![left_text, right_text]).style(style).height(3));
            } else {
                let left_text = Text::from(vec![
                    Line::raw(""),
                    Line::raw(format!("  {}", left)),
                    Line::raw(""),
                ]);
                let right_text = Text::from(vec![Line::raw(""), Line::raw(right), Line::raw("")]);
                out.push(Row::new(vec![left_text, right_text]).style(style).height(3));
            }
        }

        out
    };

    let header = Row::new(vec!["Command", "Count"])
        .style(
            Style::default()
                .fg(THEME.table_header)
                .add_modifier(Modifier::BOLD),
        )
        .height(3)
        .top_margin(1);

    let table = Table::new(
        rows,
        [Constraint::Percentage(70), Constraint::Percentage(30)],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(THEME.table_border)),
    )
    .column_spacing(3)
    .row_highlight_style(
        Style::default()
            .fg(THEME.table_active_fg)
            .bg(THEME.table_active_bg)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("");

    let mut state = TableState::default();
    if !data.is_empty() {
        state.select(Some(selected + 1));
    }
    if is_focused {
        frame.render_stateful_widget(table, area, &mut state);
    } else {
        frame.render_widget(table, area);
    }
}

fn render_chart(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    data: &[Vec<String>],
    progress: f32,
    selected_index: usize,
    is_focused: bool,
) {
    if data.is_empty() {
        let empty = Paragraph::new("No data")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(Style::default().fg(THEME.gold)),
            )
            .style(Style::default().fg(THEME.legend_dim));
        frame.render_widget(empty, area);
        return;
    }

    let max_legend_height = area.height.saturating_sub(6);
    let legend_height = if max_legend_height == 0 {
        0
    } else {
        (data.len() as u16 + 1).min(max_legend_height).max(3)
    };
    let (pie_area, legend_area) = if legend_height > 0 {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(6), Constraint::Length(legend_height)])
            .split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };

    let mut labels: Vec<String> = Vec::new();
    let mut values: Vec<f64> = Vec::new();

    for row in data {
        let label = row.get(0).cloned().unwrap_or_default();
        let value = row.get(1).and_then(|v| parse_leading_count(v)).unwrap_or(0) as f64;
        labels.push(label);
        values.push(value);
    }

    let colors = THEME.chart_palette;

    let anim = progress.clamp(0.05, 1.0) as f64;
    let slices: Vec<PieSlice<'_>> = labels
        .iter()
        .enumerate()
        .map(|(idx, label)| {
            let value = values.get(idx).copied().unwrap_or(0.0) * anim;
            let mut color = colors[idx % colors.len()];
            if idx == selected_index && is_focused {
                color = brighten_color(color, 40);
            } else if is_focused {
                color = dim_color(color, 20);
            }
            PieSlice::new(label.as_str(), value, color)
        })
        .collect();

    let piechart = PieChart::new(slices)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(THEME.gold)),
        )
        .show_legend(false)
        .show_percentages(false)
        .legend_marker(symbols::LEGEND_MARKER_CIRCLE)
        .pie_char(symbols::PIE_CHAR_BLOCK)
        .resolution(Resolution::Braille);

    frame.render_widget(piechart, pie_area);

    if let Some(legend_area) = legend_area {
        render_pie_legend(
            frame,
            legend_area,
            &labels,
            &values,
            &colors,
            selected_index,
            is_focused,
        );
    }
}

fn parse_leading_count(value: &str) -> Option<u64> {
    let trimmed = value.trim();
    let digits: String = trimmed.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse::<u64>().ok()
    }
}

fn brighten_color(color: Color, amount: u8) -> Color {
    match color {
        Color::Rgb(r, g, b) => Color::Rgb(
            r.saturating_add(amount),
            g.saturating_add(amount),
            b.saturating_add(amount),
        ),
        _ => color,
    }
}

fn dim_color(color: Color, amount: u8) -> Color {
    match color {
        Color::Rgb(r, g, b) => Color::Rgb(
            r.saturating_sub(amount),
            g.saturating_sub(amount),
            b.saturating_sub(amount),
        ),
        _ => color,
    }
}

fn chart_len(tables: Option<&OverviewTables>, chart_index: usize) -> usize {
    let tables = match tables {
        Some(t) => t,
        None => return 0,
    };

    match chart_index {
        0 => tables.top_commands.len(),
        1 => tables.top_unsuccessful.len(),
        _ => tables.mistyped_commands.len(),
    }
}

fn table_len_filtered(tables: Option<&OverviewTables>, table_index: usize, filter: &str) -> usize {
    let tables = match tables {
        Some(t) => t,
        None => return 0,
    };

    let data = match table_index {
        0 => &tables.top_commands,
        1 => &tables.top_unsuccessful,
        _ => &tables.mistyped_commands,
    };

    let filter = filter.trim().to_lowercase();
    if filter.is_empty() {
        return data.len();
    }

    data.iter()
        .filter(|row| {
            row.get(0)
                .map(|cmd| cmd.to_lowercase().contains(&filter))
                .unwrap_or(false)
        })
        .count()
}

fn filter_sort_rows(data: &[Vec<String>], filter: &str, asc: bool) -> Vec<Vec<String>> {
    let filter = filter.trim().to_lowercase();
    let mut rows: Vec<Vec<String>> = data
        .iter()
        .filter(|row| {
            if filter.is_empty() {
                return true;
            }
            row.get(0)
                .map(|cmd| cmd.to_lowercase().contains(&filter))
                .unwrap_or(false)
        })
        .cloned()
        .collect();

    rows.sort_by(|a, b| {
        let a_cmd = a.get(0).map(String::as_str).unwrap_or("");
        let b_cmd = b.get(0).map(String::as_str).unwrap_or("");
        let ord = a_cmd.to_lowercase().cmp(&b_cmd.to_lowercase());
        if asc {
            ord
        } else {
            ord.reverse()
        }
    });

    rows
}

fn render_filter_prompt(frame: &mut Frame, area: Rect, buffer: &str) {
    let width = area.width.saturating_sub(4).min(60);
    let height = 3;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let rect = Rect {
        x,
        y,
        width,
        height,
    };

    let text = format!("/ filter: {}", buffer);
    let prompt = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Filter")
                .border_style(Style::default().fg(THEME.gold)),
        )
        .style(Style::default().fg(THEME.gold_bright));
    frame.render_widget(prompt, rect);
}

fn render_recent_table(frame: &mut Frame, area: Rect, data: &[Vec<String>]) {
    let rows: Vec<Row> = if data.is_empty() {
        vec![Row::new(vec![
            "No recent commands".to_string(),
            "".to_string(),
            "".to_string(),
        ])]
    } else {
        data.iter()
            .map(|row| {
                let cmd = row.get(0).cloned().unwrap_or_default();
                let status = row.get(1).cloned().unwrap_or_default();
                let when = row.get(2).cloned().unwrap_or_default();
                Row::new(vec![cmd, status, when]).height(1)
            })
            .collect()
    };

    let header = Row::new(vec!["Command", "Status", "When"])
        .style(
            Style::default()
                .fg(THEME.table_header)
                .add_modifier(Modifier::BOLD),
        )
        .height(1);

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(55),
            Constraint::Percentage(20),
            Constraint::Percentage(25),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Recent Commands")
            .border_style(Style::default().fg(THEME.table_border)),
    )
    .column_spacing(2);

    frame.render_widget(table, area);
}

fn render_intrusion_table(frame: &mut Frame, area: Rect, data: &[Vec<String>]) {
    let rows: Vec<Row> = if data.is_empty() {
        vec![Row::new(vec![
            Cell::new(""),
            Cell::new("No suspicious commands detected"),
            Cell::new(""),
            Cell::new(""),
            Cell::new(""),
            Cell::new(""),
        ])]
    } else {
        data.iter()
            .map(|row| {
                let cmd = row.get(0).cloned().unwrap_or_default();
                let score_str = row.get(1).cloned().unwrap_or_default();
                let user = row.get(2).cloned().unwrap_or_default();
                let reason = row.get(3).cloned().unwrap_or_default();
                let when = row.get(4).cloned().unwrap_or_default();
                let score = score_str.parse::<i32>().unwrap_or(0);
                let indicator = Cell::from(Span::styled(
                    "■",
                    Style::default().fg(score_to_color(score)),
                ));
                Row::new(vec![
                    indicator,
                    Cell::new(cmd),
                    Cell::new(score_str),
                    Cell::new(user),
                    Cell::new(reason),
                    Cell::new(when),
                ])
                .height(1)
            })
            .collect()
    };

    let header = Row::new(vec!["", "Command", "Score", "User", "Reason", "When"])
        .style(
            Style::default()
                .fg(THEME.table_header)
                .add_modifier(Modifier::BOLD),
        )
        .height(1);

    let table = Table::new(
        rows,
        [
            Constraint::Length(2),
            Constraint::Percentage(32),
            Constraint::Percentage(8),
            Constraint::Percentage(12),
            Constraint::Percentage(26),
            Constraint::Percentage(18),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Potential Intrusion Signals")
            .border_style(Style::default().fg(THEME.table_border)),
    )
    .column_spacing(2);

    frame.render_widget(table, area);
}

fn render_pie_legend(
    frame: &mut Frame,
    area: Rect,
    labels: &[String],
    values: &[f64],
    colors: &[Color],
    selected_index: usize,
    is_focused: bool,
) {
    let total: f64 = values.iter().copied().sum();
    let max_label = area.width.saturating_sub(10) as usize;
    let visible = area.height.saturating_sub(2).max(1) as usize;
    let total_items = labels.len();
    let mut start = 0usize;

    if total_items > visible {
        if selected_index >= visible {
            start = selected_index + 1 - visible;
        }
        let max_start = total_items.saturating_sub(visible);
        if start > max_start {
            start = max_start;
        }
    }

    let end = (start + visible).min(total_items);

    let lines: Vec<Line> = labels
        .iter()
        .enumerate()
        .skip(start)
        .take(visible)
        .map(|(idx, label)| {
            let color = colors[idx % colors.len()];
            let percent = if total > 0.0 {
                (values.get(idx).copied().unwrap_or(0.0) / total) * 100.0
            } else {
                0.0
            };

            let marker_style = if is_focused && idx == selected_index {
                Style::default()
                    .fg(brighten_color(color, 40))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(color)
            };

            let text_style = if is_focused && idx == selected_index {
                Style::default()
                    .fg(brighten_color(color, 40))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(THEME.legend_text)
            };

            let trimmed_label: String = label.chars().take(max_label).collect();
            let label_text = format!("{} ({:.0}%)", trimmed_label, percent);

            Line::from(vec![
                Span::styled(symbols::LEGEND_MARKER_CIRCLE, marker_style),
                Span::raw(" "),
                Span::styled(label_text, text_style),
            ])
        })
        .collect();

    let title = if total_items > visible {
        format!("Legend {}-{} / {}", start + 1, end, total_items)
    } else {
        "Legend".to_string()
    };
    let legend = Paragraph::new(Text::from(lines)).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(THEME.gold)),
    );
    frame.render_widget(legend, area);
}

fn refresh_overview_state(app: &mut AppState) -> Result<(), String> {
    let tables = get_overview_tables().map_err(|e| format!("Failed to load overview: {}", e))?;
    let recent = get_recent_entries(10).map_err(|e| format!("Failed to load recent: {}", e))?;
    let recent_for_ids =
        get_recent_entries(200).map_err(|e| format!("Failed to load recent: {}", e))?;
    app.overview = Some(tables);
    app.recent_rows = build_recent_rows(&recent);
    app.intrusion_rows = build_intrusion_rows(&detect_intrusions(&recent_for_ids));
    app.last_refresh = Instant::now();
    Ok(())
}

fn build_recent_rows(entries: &[Entry]) -> Vec<Vec<String>> {
    entries
        .iter()
        .map(|entry| {
            let status = match &entry.status {
                CommandStatus::Success => "Success".to_string(),
                CommandStatus::Unknown => "Unknown".to_string(),
                CommandStatus::Error(code) => format!("Error({})", code),
            };
            let when = format_relative_time(entry.timestamp);
            vec![entry.command.clone(), status, when]
        })
        .collect()
}

fn build_intrusion_rows(findings: &[IntrusionFinding]) -> Vec<Vec<String>> {
    findings
        .iter()
        .take(10)
        .map(|finding| {
            let when = format_relative_time(finding.timestamp);
            let reason = finding.reasons.join(", ");
            vec![
                finding.command.clone(),
                finding.score.to_string(),
                format!("{} ({})", finding.user, finding.user_type),
                reason,
                when,
            ]
        })
        .collect()
}

fn format_relative_time(timestamp: i64) -> String {
    let now = Utc::now().timestamp();
    let diff = now.saturating_sub(timestamp);
    if diff < 60 {
        "just now".to_string()
    } else if diff < 3600 {
        format!("{}m ago", diff / 60)
    } else if diff < 86_400 {
        format!("{}h ago", diff / 3600)
    } else if diff < 604_800 {
        format!("{}d ago", diff / 86_400)
    } else {
        format!("{}w ago", diff / 604_800)
    }
}

fn score_to_color(score: i32) -> Color {
    let clamped = score.clamp(1, 20) as f32;
    let t = (clamped - 1.0) / 19.0;
    let r = 220u8;
    let g = (220.0 - 140.0 * t).round().max(0.0) as u8;
    let b = 0u8;
    Color::Rgb(r, g, b)
}

fn basket_logo_text(phase: usize) -> Text<'static> {
    let colors = THEME.logo_colors;

    let lines = [
        "░████████      ░███      ░██████   ░██     ░██ ░██████████ ░██████████",
        "░██    ░██    ░██░██    ░██   ░██  ░██    ░██  ░██             ░██    ",
        "░██    ░██   ░██  ░██  ░██         ░██   ░██   ░██             ░██    ",
        "░████████   ░█████████  ░████████  ░███████    ░█████████      ░██    ",
        "░██     ░██ ░██    ░██         ░██ ░██   ░██   ░██             ░██    ",
        "░██     ░██ ░██    ░██  ░██   ░██  ░██    ░██  ░██             ░██    ",
        "░█████████  ░██    ░██   ░██████   ░██     ░██ ░██████████     ░██    ",
    ];

    let styled_lines: Vec<Line> = lines
        .iter()
        .enumerate()
        .map(|(idx, line)| {
            let color = colors[(idx + phase) % colors.len()];
            let style = Style::default().fg(color).add_modifier(Modifier::BOLD);
            Line::from(Span::styled(line.to_string(), style))
        })
        .collect();

    Text::from(styled_lines)
}
