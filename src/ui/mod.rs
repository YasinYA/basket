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
use ratatui::widgets::{
    Block, Borders, Cell, List, ListItem, Paragraph, Row, Table, TableState, Wrap,
};
use ratatui::Terminal;
use tui_piechart::{symbols, PieChart, PieSlice, Resolution};

use crate::analysis::{get_overview_tables, OverviewTables};
use crate::ids::{detect_intrusions, IntrusionFinding};
use crate::runtime::realtime::save_realtime_commands;
use crate::storage::config::{load_config, save_config, AppConfig};
use crate::storage::db::{get_latest_entry_for_command, get_recent_entries, CommandStatus, Entry};

#[derive(Copy, Clone)]
struct Theme {
    name: &'static str,
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

const THEMES: [Theme; 15] = [
    Theme {
        name: "Aurora",
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
    },
    Theme {
        name: "Solar",
        gold: Color::Rgb(255, 170, 60),
        gold_bright: Color::Rgb(255, 210, 140),
        menu_inactive: Color::White,
        menu_active: Color::Rgb(255, 170, 60),
        menu_border: Color::Rgb(255, 170, 60),
        footer_border: Color::Rgb(255, 170, 60),
        header_text: Color::Rgb(255, 210, 140),
        table_border: Color::Rgb(255, 170, 60),
        table_header: Color::Rgb(255, 200, 120),
        table_row_even: Color::White,
        table_row_odd: Color::Gray,
        table_active_bg: Color::Rgb(255, 170, 60),
        table_active_fg: Color::White,
        logo_colors: [
            Color::Rgb(255, 220, 160),
            Color::Rgb(255, 200, 120),
            Color::Rgb(255, 170, 60),
            Color::Rgb(230, 140, 40),
            Color::Rgb(255, 170, 60),
            Color::Rgb(255, 200, 120),
        ],
        chart_palette: [
            Color::Rgb(255, 170, 60),
            Color::Rgb(230, 140, 40),
            Color::Rgb(255, 200, 120),
            Color::Rgb(255, 220, 160),
            Color::Rgb(210, 120, 30),
            Color::Rgb(255, 190, 90),
        ],
        legend_text: Color::Gray,
        legend_dim: Color::DarkGray,
    },
    Theme {
        name: "Ocean",
        gold: Color::Rgb(60, 170, 200),
        gold_bright: Color::Rgb(140, 220, 235),
        menu_inactive: Color::White,
        menu_active: Color::Rgb(60, 170, 200),
        menu_border: Color::Rgb(60, 170, 200),
        footer_border: Color::Rgb(60, 170, 200),
        header_text: Color::Rgb(140, 220, 235),
        table_border: Color::Rgb(60, 170, 200),
        table_header: Color::Rgb(120, 210, 230),
        table_row_even: Color::White,
        table_row_odd: Color::Gray,
        table_active_bg: Color::Rgb(60, 170, 200),
        table_active_fg: Color::White,
        logo_colors: [
            Color::Rgb(170, 235, 245),
            Color::Rgb(140, 220, 235),
            Color::Rgb(90, 190, 220),
            Color::Rgb(60, 170, 200),
            Color::Rgb(90, 190, 220),
            Color::Rgb(140, 220, 235),
        ],
        chart_palette: [
            Color::Rgb(60, 170, 200),
            Color::Rgb(40, 140, 190),
            Color::Rgb(120, 210, 230),
            Color::Rgb(170, 235, 245),
            Color::Rgb(80, 160, 210),
            Color::Rgb(100, 200, 225),
        ],
        legend_text: Color::Gray,
        legend_dim: Color::DarkGray,
    },
    Theme {
        name: "Forest",
        gold: Color::Rgb(70, 170, 90),
        gold_bright: Color::Rgb(150, 220, 165),
        menu_inactive: Color::White,
        menu_active: Color::Rgb(70, 170, 90),
        menu_border: Color::Rgb(70, 170, 90),
        footer_border: Color::Rgb(70, 170, 90),
        header_text: Color::Rgb(150, 220, 165),
        table_border: Color::Rgb(70, 170, 90),
        table_header: Color::Rgb(130, 205, 150),
        table_row_even: Color::White,
        table_row_odd: Color::Gray,
        table_active_bg: Color::Rgb(70, 170, 90),
        table_active_fg: Color::White,
        logo_colors: [
            Color::Rgb(180, 235, 195),
            Color::Rgb(150, 220, 165),
            Color::Rgb(110, 190, 130),
            Color::Rgb(70, 170, 90),
            Color::Rgb(110, 190, 130),
            Color::Rgb(150, 220, 165),
        ],
        chart_palette: [
            Color::Rgb(70, 170, 90),
            Color::Rgb(50, 140, 70),
            Color::Rgb(130, 205, 150),
            Color::Rgb(180, 235, 195),
            Color::Rgb(90, 185, 115),
            Color::Rgb(120, 200, 140),
        ],
        legend_text: Color::Gray,
        legend_dim: Color::DarkGray,
    },
    Theme {
        name: "Ember",
        gold: Color::Rgb(210, 80, 40),
        gold_bright: Color::Rgb(255, 170, 140),
        menu_inactive: Color::White,
        menu_active: Color::Rgb(210, 80, 40),
        menu_border: Color::Rgb(210, 80, 40),
        footer_border: Color::Rgb(210, 80, 40),
        header_text: Color::Rgb(255, 170, 140),
        table_border: Color::Rgb(210, 80, 40),
        table_header: Color::Rgb(240, 150, 120),
        table_row_even: Color::White,
        table_row_odd: Color::Gray,
        table_active_bg: Color::Rgb(210, 80, 40),
        table_active_fg: Color::White,
        logo_colors: [
            Color::Rgb(255, 200, 170),
            Color::Rgb(255, 170, 140),
            Color::Rgb(235, 120, 90),
            Color::Rgb(210, 80, 40),
            Color::Rgb(235, 120, 90),
            Color::Rgb(255, 170, 140),
        ],
        chart_palette: [
            Color::Rgb(210, 80, 40),
            Color::Rgb(180, 60, 35),
            Color::Rgb(240, 150, 120),
            Color::Rgb(255, 200, 170),
            Color::Rgb(205, 110, 60),
            Color::Rgb(230, 130, 90),
        ],
        legend_text: Color::Gray,
        legend_dim: Color::DarkGray,
    },
    Theme {
        name: "Slate",
        gold: Color::Rgb(120, 140, 170),
        gold_bright: Color::Rgb(190, 205, 225),
        menu_inactive: Color::White,
        menu_active: Color::Rgb(120, 140, 170),
        menu_border: Color::Rgb(120, 140, 170),
        footer_border: Color::Rgb(120, 140, 170),
        header_text: Color::Rgb(190, 205, 225),
        table_border: Color::Rgb(120, 140, 170),
        table_header: Color::Rgb(175, 195, 215),
        table_row_even: Color::White,
        table_row_odd: Color::Gray,
        table_active_bg: Color::Rgb(120, 140, 170),
        table_active_fg: Color::White,
        logo_colors: [
            Color::Rgb(210, 220, 235),
            Color::Rgb(190, 205, 225),
            Color::Rgb(150, 170, 200),
            Color::Rgb(120, 140, 170),
            Color::Rgb(150, 170, 200),
            Color::Rgb(190, 205, 225),
        ],
        chart_palette: [
            Color::Rgb(120, 140, 170),
            Color::Rgb(95, 115, 150),
            Color::Rgb(175, 195, 215),
            Color::Rgb(210, 220, 235),
            Color::Rgb(135, 155, 185),
            Color::Rgb(160, 180, 205),
        ],
        legend_text: Color::Gray,
        legend_dim: Color::DarkGray,
    },
    Theme {
        name: "Citrus",
        gold: Color::Rgb(200, 200, 70),
        gold_bright: Color::Rgb(240, 235, 150),
        menu_inactive: Color::White,
        menu_active: Color::Rgb(200, 200, 70),
        menu_border: Color::Rgb(200, 200, 70),
        footer_border: Color::Rgb(200, 200, 70),
        header_text: Color::Rgb(240, 235, 150),
        table_border: Color::Rgb(200, 200, 70),
        table_header: Color::Rgb(230, 220, 130),
        table_row_even: Color::White,
        table_row_odd: Color::Gray,
        table_active_bg: Color::Rgb(200, 200, 70),
        table_active_fg: Color::White,
        logo_colors: [
            Color::Rgb(250, 245, 185),
            Color::Rgb(240, 235, 150),
            Color::Rgb(220, 215, 110),
            Color::Rgb(200, 200, 70),
            Color::Rgb(220, 215, 110),
            Color::Rgb(240, 235, 150),
        ],
        chart_palette: [
            Color::Rgb(200, 200, 70),
            Color::Rgb(170, 175, 60),
            Color::Rgb(230, 220, 130),
            Color::Rgb(250, 245, 185),
            Color::Rgb(210, 205, 95),
            Color::Rgb(220, 215, 110),
        ],
        legend_text: Color::Gray,
        legend_dim: Color::DarkGray,
    },
    Theme {
        name: "Berry",
        gold: Color::Rgb(180, 80, 140),
        gold_bright: Color::Rgb(230, 160, 200),
        menu_inactive: Color::White,
        menu_active: Color::Rgb(180, 80, 140),
        menu_border: Color::Rgb(180, 80, 140),
        footer_border: Color::Rgb(180, 80, 140),
        header_text: Color::Rgb(230, 160, 200),
        table_border: Color::Rgb(180, 80, 140),
        table_header: Color::Rgb(215, 145, 190),
        table_row_even: Color::White,
        table_row_odd: Color::Gray,
        table_active_bg: Color::Rgb(180, 80, 140),
        table_active_fg: Color::White,
        logo_colors: [
            Color::Rgb(245, 195, 225),
            Color::Rgb(230, 160, 200),
            Color::Rgb(200, 120, 170),
            Color::Rgb(180, 80, 140),
            Color::Rgb(200, 120, 170),
            Color::Rgb(230, 160, 200),
        ],
        chart_palette: [
            Color::Rgb(180, 80, 140),
            Color::Rgb(150, 60, 120),
            Color::Rgb(215, 145, 190),
            Color::Rgb(245, 195, 225),
            Color::Rgb(185, 100, 160),
            Color::Rgb(205, 130, 180),
        ],
        legend_text: Color::Gray,
        legend_dim: Color::DarkGray,
    },
    Theme {
        name: "Dawn",
        gold: Color::Rgb(255, 120, 90),
        gold_bright: Color::Rgb(255, 195, 175),
        menu_inactive: Color::White,
        menu_active: Color::Rgb(255, 120, 90),
        menu_border: Color::Rgb(255, 120, 90),
        footer_border: Color::Rgb(255, 120, 90),
        header_text: Color::Rgb(255, 195, 175),
        table_border: Color::Rgb(255, 120, 90),
        table_header: Color::Rgb(255, 170, 150),
        table_row_even: Color::White,
        table_row_odd: Color::Gray,
        table_active_bg: Color::Rgb(255, 120, 90),
        table_active_fg: Color::White,
        logo_colors: [
            Color::Rgb(255, 215, 200),
            Color::Rgb(255, 195, 175),
            Color::Rgb(255, 150, 120),
            Color::Rgb(255, 120, 90),
            Color::Rgb(255, 150, 120),
            Color::Rgb(255, 195, 175),
        ],
        chart_palette: [
            Color::Rgb(255, 120, 90),
            Color::Rgb(225, 95, 75),
            Color::Rgb(255, 170, 150),
            Color::Rgb(255, 215, 200),
            Color::Rgb(240, 130, 105),
            Color::Rgb(250, 150, 130),
        ],
        legend_text: Color::Gray,
        legend_dim: Color::DarkGray,
    },
    Theme {
        name: "Moss",
        gold: Color::Rgb(90, 150, 120),
        gold_bright: Color::Rgb(170, 210, 190),
        menu_inactive: Color::White,
        menu_active: Color::Rgb(90, 150, 120),
        menu_border: Color::Rgb(90, 150, 120),
        footer_border: Color::Rgb(90, 150, 120),
        header_text: Color::Rgb(170, 210, 190),
        table_border: Color::Rgb(90, 150, 120),
        table_header: Color::Rgb(150, 195, 175),
        table_row_even: Color::White,
        table_row_odd: Color::Gray,
        table_active_bg: Color::Rgb(90, 150, 120),
        table_active_fg: Color::White,
        logo_colors: [
            Color::Rgb(205, 230, 215),
            Color::Rgb(170, 210, 190),
            Color::Rgb(130, 180, 155),
            Color::Rgb(90, 150, 120),
            Color::Rgb(130, 180, 155),
            Color::Rgb(170, 210, 190),
        ],
        chart_palette: [
            Color::Rgb(90, 150, 120),
            Color::Rgb(70, 125, 95),
            Color::Rgb(150, 195, 175),
            Color::Rgb(205, 230, 215),
            Color::Rgb(110, 165, 140),
            Color::Rgb(130, 180, 155),
        ],
        legend_text: Color::Gray,
        legend_dim: Color::DarkGray,
    },
    Theme {
        name: "Denim",
        gold: Color::Rgb(80, 120, 190),
        gold_bright: Color::Rgb(165, 195, 235),
        menu_inactive: Color::White,
        menu_active: Color::Rgb(80, 120, 190),
        menu_border: Color::Rgb(80, 120, 190),
        footer_border: Color::Rgb(80, 120, 190),
        header_text: Color::Rgb(165, 195, 235),
        table_border: Color::Rgb(80, 120, 190),
        table_header: Color::Rgb(145, 180, 220),
        table_row_even: Color::White,
        table_row_odd: Color::Gray,
        table_active_bg: Color::Rgb(80, 120, 190),
        table_active_fg: Color::White,
        logo_colors: [
            Color::Rgb(200, 220, 245),
            Color::Rgb(165, 195, 235),
            Color::Rgb(120, 160, 210),
            Color::Rgb(80, 120, 190),
            Color::Rgb(120, 160, 210),
            Color::Rgb(165, 195, 235),
        ],
        chart_palette: [
            Color::Rgb(80, 120, 190),
            Color::Rgb(60, 95, 160),
            Color::Rgb(145, 180, 220),
            Color::Rgb(200, 220, 245),
            Color::Rgb(100, 140, 200),
            Color::Rgb(120, 160, 210),
        ],
        legend_text: Color::Gray,
        legend_dim: Color::DarkGray,
    },
    Theme {
        name: "Copper",
        gold: Color::Rgb(195, 120, 70),
        gold_bright: Color::Rgb(235, 190, 155),
        menu_inactive: Color::White,
        menu_active: Color::Rgb(195, 120, 70),
        menu_border: Color::Rgb(195, 120, 70),
        footer_border: Color::Rgb(195, 120, 70),
        header_text: Color::Rgb(235, 190, 155),
        table_border: Color::Rgb(195, 120, 70),
        table_header: Color::Rgb(220, 170, 135),
        table_row_even: Color::White,
        table_row_odd: Color::Gray,
        table_active_bg: Color::Rgb(195, 120, 70),
        table_active_fg: Color::White,
        logo_colors: [
            Color::Rgb(245, 215, 190),
            Color::Rgb(235, 190, 155),
            Color::Rgb(210, 150, 100),
            Color::Rgb(195, 120, 70),
            Color::Rgb(210, 150, 100),
            Color::Rgb(235, 190, 155),
        ],
        chart_palette: [
            Color::Rgb(195, 120, 70),
            Color::Rgb(165, 100, 60),
            Color::Rgb(220, 170, 135),
            Color::Rgb(245, 215, 190),
            Color::Rgb(190, 135, 85),
            Color::Rgb(205, 150, 105),
        ],
        legend_text: Color::Gray,
        legend_dim: Color::DarkGray,
    },
    Theme {
        name: "Lagoon",
        gold: Color::Rgb(60, 160, 170),
        gold_bright: Color::Rgb(145, 215, 220),
        menu_inactive: Color::White,
        menu_active: Color::Rgb(60, 160, 170),
        menu_border: Color::Rgb(60, 160, 170),
        footer_border: Color::Rgb(60, 160, 170),
        header_text: Color::Rgb(145, 215, 220),
        table_border: Color::Rgb(60, 160, 170),
        table_header: Color::Rgb(125, 200, 210),
        table_row_even: Color::White,
        table_row_odd: Color::Gray,
        table_active_bg: Color::Rgb(60, 160, 170),
        table_active_fg: Color::White,
        logo_colors: [
            Color::Rgb(180, 235, 235),
            Color::Rgb(145, 215, 220),
            Color::Rgb(100, 190, 200),
            Color::Rgb(60, 160, 170),
            Color::Rgb(100, 190, 200),
            Color::Rgb(145, 215, 220),
        ],
        chart_palette: [
            Color::Rgb(60, 160, 170),
            Color::Rgb(45, 130, 145),
            Color::Rgb(125, 200, 210),
            Color::Rgb(180, 235, 235),
            Color::Rgb(85, 175, 185),
            Color::Rgb(105, 190, 200),
        ],
        legend_text: Color::Gray,
        legend_dim: Color::DarkGray,
    },
    Theme {
        name: "Sand",
        gold: Color::Rgb(200, 170, 110),
        gold_bright: Color::Rgb(235, 215, 175),
        menu_inactive: Color::White,
        menu_active: Color::Rgb(200, 170, 110),
        menu_border: Color::Rgb(200, 170, 110),
        footer_border: Color::Rgb(200, 170, 110),
        header_text: Color::Rgb(235, 215, 175),
        table_border: Color::Rgb(200, 170, 110),
        table_header: Color::Rgb(225, 200, 155),
        table_row_even: Color::White,
        table_row_odd: Color::Gray,
        table_active_bg: Color::Rgb(200, 170, 110),
        table_active_fg: Color::White,
        logo_colors: [
            Color::Rgb(245, 230, 200),
            Color::Rgb(235, 215, 175),
            Color::Rgb(215, 190, 135),
            Color::Rgb(200, 170, 110),
            Color::Rgb(215, 190, 135),
            Color::Rgb(235, 215, 175),
        ],
        chart_palette: [
            Color::Rgb(200, 170, 110),
            Color::Rgb(170, 145, 95),
            Color::Rgb(225, 200, 155),
            Color::Rgb(245, 230, 200),
            Color::Rgb(205, 180, 125),
            Color::Rgb(215, 190, 135),
        ],
        legend_text: Color::Gray,
        legend_dim: Color::DarkGray,
    },
    Theme {
        name: "Plum",
        gold: Color::Rgb(140, 90, 170),
        gold_bright: Color::Rgb(205, 175, 225),
        menu_inactive: Color::White,
        menu_active: Color::Rgb(140, 90, 170),
        menu_border: Color::Rgb(140, 90, 170),
        footer_border: Color::Rgb(140, 90, 170),
        header_text: Color::Rgb(205, 175, 225),
        table_border: Color::Rgb(140, 90, 170),
        table_header: Color::Rgb(190, 155, 215),
        table_row_even: Color::White,
        table_row_odd: Color::Gray,
        table_active_bg: Color::Rgb(140, 90, 170),
        table_active_fg: Color::White,
        logo_colors: [
            Color::Rgb(225, 205, 240),
            Color::Rgb(205, 175, 225),
            Color::Rgb(170, 130, 205),
            Color::Rgb(140, 90, 170),
            Color::Rgb(170, 130, 205),
            Color::Rgb(205, 175, 225),
        ],
        chart_palette: [
            Color::Rgb(140, 90, 170),
            Color::Rgb(115, 70, 145),
            Color::Rgb(190, 155, 215),
            Color::Rgb(225, 205, 240),
            Color::Rgb(150, 110, 190),
            Color::Rgb(170, 130, 205),
        ],
        legend_text: Color::Gray,
        legend_dim: Color::DarkGray,
    },
];

fn theme_by_index(index: usize) -> &'static Theme {
    &THEMES[index % THEMES.len()]
}

pub enum TuiExit {
    Exit,
}

enum Screen {
    Menu,
    Overview,
    Watcher,
    Error(String),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum OverviewView {
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
    theme_index: usize,
    detail_dialog: Option<DetailDialog>,
}

struct CommandDetail {
    command: String,
    status: String,
    user: String,
    time: String,
}

enum DetailDialog {
    Command(CommandDetail),
    Message(String),
}

impl AppState {
    fn new(theme_index: usize) -> Self {
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
            theme_index,
            detail_dialog: None,
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
    let config = load_config();
    let mut app = AppState::new(config.theme_index % THEMES.len());
    let tick_rate = Duration::from_millis(120);
    let refresh_rate = Duration::from_secs(1);

    loop {
        if let Err(err) = tick_app(&mut app, tick_rate, refresh_rate, refresh_overview_state) {
            app.screen = Screen::Error(err);
        }

        let theme = theme_by_index(app.theme_index);
        terminal.draw(|frame| match &app.screen {
            Screen::Menu => render_menu(frame, &menu_items, app.menu_index, app.logo_phase, theme),
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
                app.detail_dialog.as_ref(),
                theme,
            ),
            Screen::Watcher => render_watcher(frame, app.watcher_spinner, theme),
            Screen::Error(message) => render_error(frame, message, theme),
        })?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if let Some(exit_action) =
                    handle_key_event(&mut app, key.code, &menu_items, refresh_overview_state)
                {
                    return Ok(exit_action);
                }
            }
        }
    }
}

fn tick_app(
    app: &mut AppState,
    tick_rate: Duration,
    refresh_rate: Duration,
    refresh_fn: fn(&mut AppState) -> Result<(), String>,
) -> Result<(), String> {
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
            refresh_fn(app)?;
        }
        if matches!(app.screen, Screen::Watcher) {
            app.watcher_spinner = (app.watcher_spinner + 1) % spinner_frames().len();
        }
        app.last_tick = Instant::now();
    }

    Ok(())
}

fn handle_key_event(
    app: &mut AppState,
    key: KeyCode,
    menu_items: &[&str],
    refresh_fn: fn(&mut AppState) -> Result<(), String>,
) -> Option<TuiExit> {
    if app.detail_dialog.is_some() {
        match key {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('b') => {
                app.detail_dialog = None;
            }
            KeyCode::Char('q') => return Some(TuiExit::Exit),
            _ => {}
        }
        return None;
    }

    if app.filter_input {
        match key {
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
        return None;
    }

    if let KeyCode::Char('p') = key {
        app.theme_index = (app.theme_index + 1) % THEMES.len();
        let _ = save_config(&AppConfig {
            theme_index: app.theme_index,
        });
        return None;
    }

    match app.screen {
        Screen::Menu => match key {
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
                    if let Err(err) = refresh_fn(app) {
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
                _ => return Some(TuiExit::Exit),
            },
            KeyCode::Char('q') => return Some(TuiExit::Exit),
            _ => {}
        },
        Screen::Overview => match key {
            KeyCode::Char('c') => {
                app.overview_view = OverviewView::Charts;
                app.chart_progress = 0.0;
                app.chart_index = 0;
            }
            KeyCode::Char('t') => {
                app.overview_view = OverviewView::Tables;
                app.table_index = 0;
            }
            KeyCode::Enter => {
                if matches!(app.overview_view, OverviewView::Tables) {
                    if let Some(command) = selected_command_from_table(app) {
                        match get_latest_entry_for_command(&command) {
                            Ok(Some(entry)) => {
                                let detail = CommandDetail {
                                    command: entry.command,
                                    status: status_label(&entry.status),
                                    user: entry.user,
                                    time: format_relative_time(entry.timestamp),
                                };
                                app.detail_dialog = Some(DetailDialog::Command(detail));
                            }
                            Ok(None) => {
                                app.detail_dialog = Some(DetailDialog::Message(format!(
                                    "No recent entry found for \"{}\".",
                                    command
                                )));
                            }
                            Err(err) => {
                                app.detail_dialog = Some(DetailDialog::Message(format!(
                                    "Failed to load command details: {}",
                                    err
                                )));
                            }
                        }
                    }
                }
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
            KeyCode::Char('q') => return Some(TuiExit::Exit),
            _ => {}
        },
        Screen::Watcher => match key {
            KeyCode::Char('b') => {
                app.screen = Screen::Menu;
            }
            KeyCode::Char('q') => return Some(TuiExit::Exit),
            _ => {}
        },
        Screen::Error(_) => match key {
            KeyCode::Char('b') => {
                app.screen = Screen::Menu;
            }
            KeyCode::Char('q') => return Some(TuiExit::Exit),
            _ => {}
        },
    }

    None
}

fn render_menu(
    frame: &mut Frame,
    items: &[&str],
    selected: usize,
    logo_phase: usize,
    theme: &Theme,
) {
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
                .fg(theme.header_text)
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Basket")
                .border_style(Style::default().fg(theme.gold)),
        );

    let logo = Paragraph::new(basket_logo_text(logo_phase, theme)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.gold)),
    );

    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let prefix = if idx == selected { "> " } else { "  " };
            let item_style = if idx == selected {
                Style::default()
                    .fg(theme.menu_active)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.menu_inactive)
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
                .border_style(Style::default().fg(theme.menu_border)),
        )
        .highlight_style(Style::default());

    let footer = Paragraph::new(format!(
        "Use Up/Down + Enter. p: theme ({})  q: quit.",
        theme.name
    ))
    .block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(theme.footer_border)),
    )
    .style(Style::default().fg(theme.footer_border));

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
    detail_dialog: Option<&DetailDialog>,
    theme: &Theme,
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
                    theme,
                );
                render_table(
                    frame,
                    chunks[1],
                    "Most Unsuccessful Commands",
                    &top_unsuccessful,
                    table_selection[1],
                    table_index == 1,
                    theme,
                );
                render_table(
                    frame,
                    chunks[2],
                    "Most Mistyped Commands",
                    &mistyped_commands,
                    table_selection[2],
                    table_index == 2,
                    theme,
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
                    theme,
                );
                render_chart(
                    frame,
                    chunks[1],
                    "Most Unsuccessful Commands",
                    &tables.top_unsuccessful,
                    chart_progress,
                    chart_selection[1],
                    chart_index == 1,
                    theme,
                );
                render_chart(
                    frame,
                    chunks[2],
                    "Most Mistyped Commands",
                    &tables.mistyped_commands,
                    chart_progress,
                    chart_selection[2],
                    chart_index == 2,
                    theme,
                );
            }
        }
    } else {
        let empty = Paragraph::new("No overview data.").block(
            Block::default()
                .borders(Borders::ALL)
                .title("Overview")
                .border_style(Style::default().fg(theme.gold)),
        );
        frame.render_widget(empty, layout[0]);
    }

    render_recent_table(frame, layout[1], recent_rows, theme);
    render_intrusion_table(frame, layout[2], intrusion_rows, theme);

    let sort_label = if sort_asc { "A->Z" } else { "Z->A" };
    let filter_label = if filter_query.is_empty() {
        "none"
    } else {
        filter_query
    };
    let footer = match view {
        OverviewView::Tables => Paragraph::new(format!(
            "t: tables  c: charts  ←/→: table  ↑/↓: row  enter: details  s: sort({})  /: filter({})  x: clear  p: theme({})  b: back  q: quit",
            sort_label, filter_label, theme.name
        )),
        OverviewView::Charts => Paragraph::new(format!(
            "t: tables  c: charts  ←/→: chart  ↑/↓: slice  p: theme({})  b: back  q: quit",
            theme.name
        )),
    }
    .block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(theme.footer_border)),
    );
    frame.render_widget(footer, layout[3]);

    if filter_input {
        render_filter_prompt(frame, layout[0], filter_buffer, theme);
    }

    if let Some(detail) = detail_dialog {
        render_detail_dialog(frame, detail, theme);
    }
}

fn render_error(frame: &mut Frame, message: &str, theme: &Theme) {
    let size = frame.area();
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Error")
        .border_style(Style::default().fg(theme.gold));
    let paragraph = Paragraph::new(message)
        .block(block)
        .style(Style::default().fg(Color::Red));
    frame.render_widget(paragraph, size);
}

fn render_watcher(frame: &mut Frame, spinner_index: usize, theme: &Theme) {
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
                .border_style(Style::default().fg(theme.gold)),
        )
        .style(
            Style::default()
                .fg(theme.menu_inactive)
                .add_modifier(Modifier::BOLD),
        );

    let footer = Paragraph::new(format!("p: theme({})  b: back  q: quit", theme.name))
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(theme.footer_border)),
        )
        .style(Style::default().fg(theme.footer_border));

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
    theme: &Theme,
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
        .style(Style::default().fg(theme.table_border));

        let mut out = Vec::with_capacity(data.len() + 1);
        out.push(separator);

        for (idx, row) in data.iter().enumerate() {
            let left = row.get(0).cloned().unwrap_or_default();
            let right = row.get(1).cloned().unwrap_or_default();
            let style = if idx % 2 == 0 {
                Style::default()
                    .fg(theme.table_row_even)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(theme.table_row_odd)
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
                .fg(theme.table_header)
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
            .border_style(Style::default().fg(theme.table_border)),
    )
    .column_spacing(3)
    .row_highlight_style(
        Style::default()
            .fg(theme.table_active_fg)
            .bg(theme.table_active_bg)
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
    theme: &Theme,
) {
    if data.is_empty() {
        let empty = Paragraph::new("No data")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(Style::default().fg(theme.gold)),
            )
            .style(Style::default().fg(theme.legend_dim));
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

    let colors = theme.chart_palette;

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
                .border_style(Style::default().fg(theme.gold)),
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
            theme,
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

fn render_filter_prompt(frame: &mut Frame, area: Rect, buffer: &str, theme: &Theme) {
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
                .border_style(Style::default().fg(theme.gold)),
        )
        .style(Style::default().fg(theme.gold_bright));
    frame.render_widget(prompt, rect);
}

fn render_detail_dialog(frame: &mut Frame, detail: &DetailDialog, theme: &Theme) {
    let size = frame.area();
    let width = size.width.saturating_sub(6).min(76).max(30);
    let height = match detail {
        DetailDialog::Command(_) => 9,
        DetailDialog::Message(_) => 7,
    };
    let x = size.x + (size.width.saturating_sub(width)) / 2;
    let y = size.y + (size.height.saturating_sub(height)) / 2;
    let rect = Rect {
        x,
        y,
        width,
        height,
    };

    let label_style = Style::default()
        .fg(theme.table_header)
        .add_modifier(Modifier::BOLD);
    let dim_style = Style::default().fg(theme.legend_dim);

    let (title, content, content_style) = match detail {
        DetailDialog::Command(detail) => {
            let lines = vec![
                Line::from(vec![
                    Span::styled("Command: ", label_style),
                    Span::raw(detail.command.as_str()),
                ]),
                Line::from(vec![
                    Span::styled("Status:  ", label_style),
                    Span::raw(detail.status.as_str()),
                ]),
                Line::from(vec![
                    Span::styled("User:    ", label_style),
                    Span::raw(detail.user.as_str()),
                ]),
                Line::from(vec![
                    Span::styled("Time:    ", label_style),
                    Span::raw(detail.time.as_str()),
                ]),
                Line::raw(""),
                Line::styled("Esc/Enter to close", dim_style),
            ];
            (
                "Command Details",
                Text::from(lines),
                Style::default().fg(theme.menu_inactive),
            )
        }
        DetailDialog::Message(message) => {
            let lines = vec![
                Line::raw(message.as_str()),
                Line::raw(""),
                Line::styled("Esc/Enter to close", dim_style),
            ];
            (
                "Command Details",
                Text::from(lines),
                Style::default().fg(Color::Red),
            )
        }
    };

    let dialog = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(theme.gold)),
        )
        .style(content_style)
        .wrap(Wrap { trim: true });

    frame.render_widget(dialog, rect);
}

fn render_recent_table(frame: &mut Frame, area: Rect, data: &[Vec<String>], theme: &Theme) {
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
                .fg(theme.table_header)
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
            .border_style(Style::default().fg(theme.table_border)),
    )
    .column_spacing(2);

    frame.render_widget(table, area);
}

fn render_intrusion_table(frame: &mut Frame, area: Rect, data: &[Vec<String>], theme: &Theme) {
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
                .fg(theme.table_header)
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
            .border_style(Style::default().fg(theme.table_border)),
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
    theme: &Theme,
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
                Style::default().fg(theme.legend_text)
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
            .border_style(Style::default().fg(theme.gold)),
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
            let status = status_label(&entry.status);
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
    format_relative_time_at(timestamp, now)
}

pub fn format_relative_time_at(timestamp: i64, now: i64) -> String {
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

fn status_label(status: &CommandStatus) -> String {
    match status {
        CommandStatus::Success => "Success".to_string(),
        CommandStatus::Unknown => "Unknown".to_string(),
        CommandStatus::Error(code) => format!("Error({})", code),
    }
}

fn selected_command_from_table(app: &AppState) -> Option<String> {
    let tables = app.overview.as_ref()?;
    let (data, selection) = match app.table_index {
        0 => (&tables.top_commands, app.table_selection[0]),
        1 => (&tables.top_unsuccessful, app.table_selection[1]),
        _ => (&tables.mistyped_commands, app.table_selection[2]),
    };

    let rows = filter_sort_rows(data, &app.filter_query, app.sort_asc);
    if rows.is_empty() {
        return None;
    }

    let selected = selection.min(rows.len().saturating_sub(1));
    rows.get(selected).and_then(|row| row.get(0).cloned())
}

pub mod testing {
    use super::*;

    pub use super::OverviewView;
    pub struct TestState {
        app: AppState,
    }

    impl TestState {
        pub fn new() -> Self {
            Self {
                app: AppState::new(0),
            }
        }

        pub fn set_screen_menu(&mut self) {
            self.app.screen = Screen::Menu;
        }

        pub fn set_screen_overview(&mut self) {
            self.app.screen = Screen::Overview;
        }

        pub fn set_screen_watcher(&mut self) {
            self.app.screen = Screen::Watcher;
        }

        pub fn set_screen_error(&mut self, message: &str) {
            self.app.screen = Screen::Error(message.to_string());
        }

        pub fn set_overview(&mut self, tables: OverviewTables) {
            self.app.overview = Some(tables);
        }

        pub fn set_overview_view(&mut self, view: OverviewView) {
            self.app.overview_view = view;
        }

        pub fn set_filter_query(&mut self, query: &str) {
            self.app.filter_query = query.to_string();
        }

        pub fn set_filter_input(&mut self, enabled: bool) {
            self.app.filter_input = enabled;
        }

        pub fn set_filter_buffer(&mut self, buffer: &str) {
            self.app.filter_buffer = buffer.to_string();
        }

        pub fn set_watcher_started(&mut self, started: bool) {
            self.app.watcher_started = started;
        }

        pub fn set_last_tick_offset(&mut self, offset: Duration) {
            self.app.last_tick = Instant::now()
                .checked_sub(offset)
                .unwrap_or_else(Instant::now);
        }

        pub fn set_last_refresh_offset(&mut self, offset: Duration) {
            self.app.last_refresh = Instant::now()
                .checked_sub(offset)
                .unwrap_or_else(Instant::now);
        }

        pub fn screen(&self) -> &'static str {
            match self.app.screen {
                Screen::Menu => "menu",
                Screen::Overview => "overview",
                Screen::Watcher => "watcher",
                Screen::Error(_) => "error",
            }
        }

        pub fn menu_index(&self) -> usize {
            self.app.menu_index
        }

        pub fn theme_index(&self) -> usize {
            self.app.theme_index
        }

        pub fn set_menu_index(&mut self, idx: usize) {
            self.app.menu_index = idx;
        }

        pub fn overview_view(&self) -> OverviewView {
            self.app.overview_view
        }

        pub fn chart_index(&self) -> usize {
            self.app.chart_index
        }

        pub fn table_index(&self) -> usize {
            self.app.table_index
        }

        pub fn filter_input(&self) -> bool {
            self.app.filter_input
        }

        pub fn filter_query(&self) -> &str {
            &self.app.filter_query
        }

        pub fn filter_buffer(&self) -> &str {
            &self.app.filter_buffer
        }

        pub fn chart_selection(&self) -> [usize; 3] {
            self.app.chart_selection
        }

        pub fn table_selection(&self) -> [usize; 3] {
            self.app.table_selection
        }

        pub fn watcher_spinner(&self) -> usize {
            self.app.watcher_spinner
        }

        pub fn chart_progress(&self) -> f32 {
            self.app.chart_progress
        }

        pub fn tick(&mut self, tick_rate: Duration, refresh_rate: Duration) -> Result<(), String> {
            tick_app(
                &mut self.app,
                tick_rate,
                refresh_rate,
                refresh_stub_for_test,
            )
        }

        pub fn handle_key(&mut self, key: KeyCode, menu_items: &[&str]) -> Option<TuiExit> {
            handle_key_event(&mut self.app, key, menu_items, refresh_stub_for_test)
        }

        pub fn handle_key_with_error(
            &mut self,
            key: KeyCode,
            menu_items: &[&str],
        ) -> Option<TuiExit> {
            handle_key_event(&mut self.app, key, menu_items, refresh_error_for_test)
        }
    }

    pub fn render_menu_for_test(
        frame: &mut Frame,
        items: &[&str],
        selected: usize,
        logo_phase: usize,
    ) {
        render_menu(frame, items, selected, logo_phase, theme_by_index(0));
    }

    pub fn render_overview_for_test(
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
        render_overview(
            frame,
            tables,
            view,
            chart_progress,
            chart_index,
            chart_selection,
            table_index,
            table_selection,
            filter_query,
            sort_asc,
            filter_input,
            filter_buffer,
            recent_rows,
            intrusion_rows,
            None,
            theme_by_index(0),
        );
    }

    pub fn render_error_for_test(frame: &mut Frame, message: &str) {
        render_error(frame, message, theme_by_index(0));
    }

    pub fn render_watcher_for_test(frame: &mut Frame, spinner_index: usize) {
        render_watcher(frame, spinner_index, theme_by_index(0));
    }

    pub fn render_table_for_test(
        frame: &mut Frame,
        area: Rect,
        title: &str,
        data: &[Vec<String>],
        selected_index: usize,
        is_focused: bool,
    ) {
        render_table(
            frame,
            area,
            title,
            data,
            selected_index,
            is_focused,
            theme_by_index(0),
        );
    }

    pub fn render_chart_for_test(
        frame: &mut Frame,
        area: Rect,
        title: &str,
        data: &[Vec<String>],
        progress: f32,
        selected_index: usize,
        is_focused: bool,
    ) {
        render_chart(
            frame,
            area,
            title,
            data,
            progress,
            selected_index,
            is_focused,
            theme_by_index(0),
        );
    }

    pub fn render_filter_prompt_for_test(frame: &mut Frame, area: Rect, buffer: &str) {
        render_filter_prompt(frame, area, buffer, theme_by_index(0));
    }

    pub fn render_recent_table_for_test(frame: &mut Frame, area: Rect, data: &[Vec<String>]) {
        render_recent_table(frame, area, data, theme_by_index(0));
    }

    pub fn render_intrusion_table_for_test(frame: &mut Frame, area: Rect, data: &[Vec<String>]) {
        render_intrusion_table(frame, area, data, theme_by_index(0));
    }

    pub fn render_pie_legend_for_test(
        frame: &mut Frame,
        area: Rect,
        labels: &[String],
        values: &[f64],
        colors: &[Color],
        selected_index: usize,
        is_focused: bool,
    ) {
        render_pie_legend(
            frame,
            area,
            labels,
            values,
            colors,
            selected_index,
            is_focused,
            theme_by_index(0),
        );
    }

    pub fn parse_leading_count_for_test(value: &str) -> Option<u64> {
        parse_leading_count(value)
    }

    pub fn brighten_color_for_test(color: Color, amount: u8) -> Color {
        brighten_color(color, amount)
    }

    pub fn dim_color_for_test(color: Color, amount: u8) -> Color {
        dim_color(color, amount)
    }

    pub fn chart_len_for_test(tables: Option<&OverviewTables>, chart_index: usize) -> usize {
        chart_len(tables, chart_index)
    }

    pub fn table_len_filtered_for_test(
        tables: Option<&OverviewTables>,
        table_index: usize,
        filter: &str,
    ) -> usize {
        table_len_filtered(tables, table_index, filter)
    }

    pub fn filter_sort_rows_for_test(
        data: &[Vec<String>],
        filter: &str,
        asc: bool,
    ) -> Vec<Vec<String>> {
        filter_sort_rows(data, filter, asc)
    }

    pub fn build_recent_rows_for_test(entries: &[Entry]) -> Vec<Vec<String>> {
        build_recent_rows(entries)
    }

    pub fn build_intrusion_rows_for_test(findings: &[IntrusionFinding]) -> Vec<Vec<String>> {
        build_intrusion_rows(findings)
    }

    pub fn score_to_color_for_test(score: i32) -> Color {
        score_to_color(score)
    }

    pub fn spinner_frames_for_test() -> [&'static str; 10] {
        spinner_frames()
    }

    pub fn theme_count_for_test() -> usize {
        THEMES.len()
    }

    pub fn refresh_overview_for_test() -> Result<(usize, usize), String> {
        let mut app = AppState::new(0);
        refresh_overview_state(&mut app)?;
        Ok((app.recent_rows.len(), app.intrusion_rows.len()))
    }

    pub fn logo_text_for_test(phase: usize) -> Text<'static> {
        basket_logo_text(phase, theme_by_index(0))
    }

    fn refresh_stub_for_test(app: &mut AppState) -> Result<(), String> {
        app.overview = Some(OverviewTables {
            top_commands: vec![vec!["ls".to_string(), "2".to_string()]],
            top_unsuccessful: vec![vec!["rm".to_string(), "1".to_string()]],
            mistyped_commands: vec![vec!["git".to_string(), "1 (gti)".to_string()]],
        });
        Ok(())
    }

    fn refresh_error_for_test(_app: &mut AppState) -> Result<(), String> {
        Err("refresh failed".to_string())
    }
}

fn basket_logo_text(phase: usize, theme: &Theme) -> Text<'static> {
    let colors = theme.logo_colors;

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
