use std::error::Error;
use std::io;
use std::time::{Duration, Instant};

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
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Row, Table, TableState};
use ratatui::Terminal;
use tui_piechart::{symbols, PieChart, PieSlice, Resolution};

use crate::analyze_commands::{get_overview_tables, OverviewTables};

pub enum TuiExit {
    Exit,
    StartRealtime,
}

enum Screen {
    Menu,
    Overview,
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
    let mut exit_action = TuiExit::Exit;

    let tick_rate = Duration::from_millis(120);

    loop {
        if app.last_tick.elapsed() >= tick_rate {
            app.logo_phase = (app.logo_phase + 1) % 6;
            if matches!(app.screen, Screen::Overview)
                && matches!(app.overview_view, OverviewView::Charts)
                && app.chart_progress < 1.0
            {
                app.chart_progress = (app.chart_progress + 0.08).min(1.0);
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
            ),
            Screen::Error(message) => render_error(frame, message),
        })?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
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
                            0 => match get_overview_tables() {
                                Ok(tables) => {
                                    app.overview = Some(tables);
                                    app.screen = Screen::Overview;
                                }
                                Err(err) => {
                                    app.screen =
                                        Screen::Error(format!("Failed to load overview: {}", err));
                                }
                            },
                            1 => {
                                exit_action = TuiExit::StartRealtime;
                                return Ok(exit_action);
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
                                let len = chart_len(app.overview.as_ref(), app.table_index);
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
                                let len = chart_len(app.overview.as_ref(), app.table_index);
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

    let powered_by = Paragraph::new("Powered by")
        .style(
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL).title("Basket"));

    let logo = Paragraph::new(basket_logo_text(logo_phase)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    );

    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let colors = [
                Color::Rgb(56, 189, 248),
                Color::Rgb(34, 197, 94),
                Color::Rgb(250, 204, 21),
            ];
            let base = colors[idx % colors.len()];
            let prefix = if idx == selected { "> " } else { "  " };
            let item_style = if idx == selected {
                Style::default()
                    .fg(brighten_color(base, 40))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(base)
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
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(Style::default());

    let footer = Paragraph::new("Use Up/Down + Enter. Press q to quit.")
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Color::Blue)),
        )
        .style(Style::default().fg(Color::Blue));

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
) {
    let size = frame.area();
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(10), Constraint::Length(3)])
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

                render_table(
                    frame,
                    chunks[0],
                    "Command Occurrences",
                    &tables.top_commands,
                    table_selection[0],
                    table_index == 0,
                );
                render_table(
                    frame,
                    chunks[1],
                    "Most Unsuccessful Commands",
                    &tables.top_unsuccessful,
                    table_selection[1],
                    table_index == 1,
                );
                render_table(
                    frame,
                    chunks[2],
                    "Most Mistyped Commands",
                    &tables.mistyped_commands,
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
        let empty = Paragraph::new("No overview data.")
            .block(Block::default().borders(Borders::ALL).title("Overview"));
        frame.render_widget(empty, layout[0]);
    }

    let footer_text = match view {
        OverviewView::Tables => "t: tables  c: charts  ←/→: table  ↑/↓: row  b: back  q: quit",
        OverviewView::Charts => "t: tables  c: charts  ←/→: chart  ↑/↓: slice  b: back  q: quit",
    };
    let footer = Paragraph::new(footer_text).block(Block::default().borders(Borders::TOP));
    frame.render_widget(footer, layout[1]);
}

fn render_error(frame: &mut Frame, message: &str) {
    let size = frame.area();
    let block = Block::default().borders(Borders::ALL).title("Error");
    let paragraph = Paragraph::new(message)
        .block(block)
        .style(Style::default().fg(Color::Red));
    frame.render_widget(paragraph, size);
}

fn render_table(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    data: &[Vec<String>],
    selected_index: usize,
    is_focused: bool,
) {
    let rows: Vec<Row> = if data.is_empty() {
        vec![Row::new(vec!["(no data)".to_string(), "".to_string()])]
    } else {
        data.iter()
            .enumerate()
            .map(|(idx, row)| {
                let left = row.get(0).cloned().unwrap_or_default();
                let right = row.get(1).cloned().unwrap_or_default();
                let style = if idx % 2 == 0 {
                    Style::default().fg(Color::White)
                } else {
                    Style::default().fg(Color::Gray)
                };
                Row::new(vec![left, right]).style(style)
            })
            .collect()
    };

    let header = Row::new(vec!["Command", "Count"]).style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );

    let table = Table::new(
        rows,
        [Constraint::Percentage(70), Constraint::Percentage(30)],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(Color::Blue)),
    )
    .row_highlight_style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("➤ ");

    let mut state = TableState::default();
    if !data.is_empty() {
        state.select(Some(selected_index.min(data.len().saturating_sub(1))));
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
            .block(Block::default().borders(Borders::ALL).title(title))
            .style(Style::default().fg(Color::DarkGray));
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

    let colors = [
        Color::Rgb(56, 189, 248),
        Color::Rgb(34, 197, 94),
        Color::Rgb(250, 204, 21),
        Color::Rgb(244, 114, 182),
        Color::Rgb(59, 130, 246),
        Color::Rgb(251, 146, 60),
    ];

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
        .block(Block::default().borders(Borders::ALL).title(title))
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
                Style::default().fg(Color::Gray)
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
    let legend = Paragraph::new(Text::from(lines))
        .block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(legend, area);
}

fn basket_logo_text(phase: usize) -> Text<'static> {
    let colors = [
        Color::Rgb(56, 189, 248),
        Color::Rgb(14, 165, 233),
        Color::Rgb(2, 132, 199),
        Color::Rgb(3, 105, 161),
        Color::Rgb(2, 132, 199),
        Color::Rgb(14, 165, 233),
    ];

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
