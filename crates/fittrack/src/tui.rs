use std::fs;
use std::io;
use std::panic;
use std::path::PathBuf;
use std::time::Duration;

use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
    MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::border;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use fittrack::{
    CompiledTraining, Exercise, ExerciseCatalog, Training, compile_document_with_catalog,
    render_json,
};

#[derive(Debug, Clone)]
pub struct TuiConfig {
    pub fit_path: PathBuf,
    pub exercise_path: PathBuf,
    pub output_path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BufferKind {
    Training,
    Exercises,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Editor,
    Dashboard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DashboardFilter {
    Year,
    Month,
    Exercise,
}

struct Buffer {
    title: &'static str,
    path: PathBuf,
    lines: Vec<String>,
    cursor_row: usize,
    cursor_col: usize,
}

impl Buffer {
    fn load(title: &'static str, path: PathBuf) -> Result<Self, String> {
        let source = fs::read_to_string(&path)
            .map_err(|err| format!("Could not read {}: {err}", path.display()))?;
        let mut lines = source.lines().map(str::to_string).collect::<Vec<_>>();
        if lines.is_empty() {
            lines.push(String::new());
        }
        Ok(Self {
            title,
            path,
            lines,
            cursor_row: 0,
            cursor_col: 0,
        })
    }

    fn source(&self) -> String {
        let mut source = self.lines.join("\n");
        source.push('\n');
        source
    }

    fn save(&self) -> Result<(), String> {
        fs::write(&self.path, self.source())
            .map_err(|err| format!("Could not write {}: {err}", self.path.display()))
    }

    fn current_line(&self) -> &str {
        self.lines
            .get(self.cursor_row)
            .map(String::as_str)
            .unwrap_or_default()
    }

    fn insert_char(&mut self, ch: char) {
        if let Some(line) = self.lines.get_mut(self.cursor_row) {
            let col = self.cursor_col.min(line.len());
            line.insert(col, ch);
            self.cursor_col = col + ch.len_utf8();
        }
    }

    fn replace_range(&mut self, start: usize, end: usize, text: &str) {
        if let Some(line) = self.lines.get_mut(self.cursor_row) {
            let start = start.min(line.len());
            let end = end.min(line.len()).max(start);
            line.replace_range(start..end, text);
            self.cursor_col = start + text.len();
        }
    }

    fn newline(&mut self) {
        let current = self.lines[self.cursor_row].clone();
        let col = self.cursor_col.min(current.len());
        let (left, right) = current.split_at(col);
        self.lines[self.cursor_row] = left.to_string();
        self.lines.insert(self.cursor_row + 1, right.to_string());
        self.cursor_row += 1;
        self.cursor_col = 0;
    }

    fn backspace(&mut self) {
        if self.cursor_col > 0 {
            if let Some(line) = self.lines.get_mut(self.cursor_row) {
                let previous = previous_boundary(line, self.cursor_col);
                line.replace_range(previous..self.cursor_col, "");
                self.cursor_col = previous;
            }
        } else if self.cursor_row > 0 {
            let removed = self.lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            self.cursor_col = self.lines[self.cursor_row].len();
            self.lines[self.cursor_row].push_str(&removed);
        }
    }

    fn move_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col = previous_boundary(self.current_line(), self.cursor_col);
        } else if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = self.current_line().len();
        }
    }

    fn move_right(&mut self) {
        let line_len = self.current_line().len();
        if self.cursor_col < line_len {
            self.cursor_col = next_boundary(self.current_line(), self.cursor_col);
        } else if self.cursor_row + 1 < self.lines.len() {
            self.cursor_row += 1;
            self.cursor_col = 0;
        }
    }

    fn move_up(&mut self) {
        if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = self.cursor_col.min(self.current_line().len());
        }
    }

    fn move_down(&mut self) {
        if self.cursor_row + 1 < self.lines.len() {
            self.cursor_row += 1;
            self.cursor_col = self.cursor_col.min(self.current_line().len());
        }
    }
}

pub fn run(config: TuiConfig) -> Result<(), String> {
    let mut app = App::load(config)?;
    enable_raw_mode().map_err(|err| err.to_string())?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture).map_err(|err| err.to_string())?;
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), DisableMouseCapture, LeaveAlternateScreen);
        original_hook(info);
    }));
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(|err| err.to_string())?;
    let result = run_loop(&mut terminal, &mut app);
    disable_raw_mode().map_err(|err| err.to_string())?;
    execute!(
        terminal.backend_mut(),
        DisableMouseCapture,
        LeaveAlternateScreen
    )
    .map_err(|err| err.to_string())?;
    terminal.show_cursor().map_err(|err| err.to_string())?;
    result
}

struct App {
    training: Buffer,
    exercises: Buffer,
    output_path: PathBuf,
    active: BufferKind,
    screen: Screen,
    compiled: Option<CompiledTraining>,
    diagnostics: Vec<Diagnostic>,
    status: String,
    completion_index: usize,
    dashboard_scroll: u16,
    dashboard_filter: DashboardFilter,
    selected_year: Option<String>,
    selected_month: Option<String>,
    selected_exercise: Option<String>,
}

struct Diagnostic {
    line: Option<usize>,
    message: String,
}

impl App {
    fn load(config: TuiConfig) -> Result<Self, String> {
        let mut app = Self {
            training: Buffer::load("training.fit", config.fit_path)?,
            exercises: Buffer::load("exercises.txt", config.exercise_path)?,
            output_path: config.output_path,
            active: BufferKind::Training,
            screen: Screen::Editor,
            compiled: None,
            diagnostics: Vec::new(),
            status:
                "F5 compile | Tab accept completion | Ctrl-N/P choose | Ctrl-S save | Ctrl-C quit"
                    .to_string(),
            completion_index: 0,
            dashboard_scroll: 0,
            dashboard_filter: DashboardFilter::Year,
            selected_year: None,
            selected_month: None,
            selected_exercise: None,
        };
        app.revalidate();
        Ok(app)
    }

    fn active_buffer(&self) -> &Buffer {
        match self.active {
            BufferKind::Training => &self.training,
            BufferKind::Exercises => &self.exercises,
        }
    }

    fn active_buffer_mut(&mut self) -> &mut Buffer {
        match self.active {
            BufferKind::Training => &mut self.training,
            BufferKind::Exercises => &mut self.exercises,
        }
    }

    fn switch_buffer(&mut self) {
        self.active = match self.active {
            BufferKind::Training => BufferKind::Exercises,
            BufferKind::Exercises => BufferKind::Training,
        };
        self.screen = Screen::Editor;
        self.completion_index = 0;
    }

    fn save_all(&mut self) {
        let result = self.training.save().and_then(|_| self.exercises.save());
        self.status = match result {
            Ok(()) => "Saved training and exercise catalog".to_string(),
            Err(err) => err,
        };
    }

    fn compile(&mut self) {
        self.revalidate();
        let Some(compiled) = self.compiled.as_ref() else {
            self.status = "Fix diagnostics before compiling".to_string();
            return;
        };
        if let Some(parent) = self.output_path.parent() {
            if let Err(err) = fs::create_dir_all(parent) {
                self.status = format!("Could not create {}: {err}", parent.display());
                return;
            }
        }
        match fs::write(&self.output_path, render_json(compiled)) {
            Ok(()) => {
                self.status = format!("Compiled {}", self.output_path.display());
                self.screen = Screen::Dashboard;
                self.dashboard_scroll = 0;
                self.ensure_dashboard_exercise();
            }
            Err(err) => {
                self.status = format!("Could not write {}: {err}", self.output_path.display())
            }
        }
    }

    fn revalidate(&mut self) {
        self.diagnostics.clear();
        let catalog = match ExerciseCatalog::parse(&self.exercises.source()) {
            Ok(catalog) => Some(catalog),
            Err(err) => {
                self.compiled = None;
                self.diagnostics.push(Diagnostic {
                    line: parse_line_no(&err),
                    message: err,
                });
                return;
            }
        };
        match compile_document_with_catalog(&self.training.source(), catalog.as_ref()) {
            Ok(compiled) => {
                self.compiled = Some(compiled);
                self.status = "Document is valid".to_string();
            }
            Err(err) => {
                self.compiled = None;
                self.diagnostics.push(Diagnostic {
                    line: parse_line_no(&err),
                    message: err,
                });
            }
        }
    }

    fn completions(&self) -> Vec<Completion> {
        match self.active {
            BufferKind::Exercises => exercise_catalog_completions(self.active_buffer()),
            BufferKind::Training => {
                training_completions(self.active_buffer(), &self.exercises.source())
            }
        }
    }

    fn accept_completion(&mut self) {
        let completions = self.completions();
        if let Some(completion) = completions.get(
            self.completion_index
                .min(completions.len().saturating_sub(1)),
        ) {
            let completion = completion.clone();
            self.active_buffer_mut().replace_range(
                completion.replace_start,
                completion.replace_end,
                &completion.insert_text,
            );
            self.completion_index = 0;
            self.revalidate();
        }
    }

    fn select_next_completion(&mut self) {
        let len = self.completions().len();
        if len > 0 {
            self.completion_index = (self.completion_index + 1) % len;
        }
    }

    fn select_previous_completion(&mut self) {
        let len = self.completions().len();
        if len > 0 {
            self.completion_index = (self.completion_index + len - 1) % len;
        }
    }

    fn filtered_trainings(&self) -> Vec<&Training> {
        let Some(compiled) = self.compiled.as_ref() else {
            return Vec::new();
        };
        compiled
            .trainings
            .iter()
            .filter(|training| {
                let year = training.date.get(0..4).unwrap_or_default();
                let month = training.date.get(5..7).unwrap_or_default();
                self.selected_year
                    .as_deref()
                    .map_or(true, |value| value == year)
                    && self
                        .selected_month
                        .as_deref()
                        .map_or(true, |value| value == month)
            })
            .collect()
    }

    fn filtered_summary(&self) -> SummaryView {
        let trainings = self.filtered_trainings();
        SummaryView {
            total_trainings: trainings.len(),
            total_sets: trainings
                .iter()
                .flat_map(|training| training.exercises.iter())
                .map(|exercise| exercise.sets.len())
                .sum(),
            total_volume_kg: trainings
                .iter()
                .map(|training| training_volume(training))
                .sum(),
            total_cardio_km: trainings
                .iter()
                .flat_map(|training| training.cardio.iter())
                .map(|cardio| cardio.distance_km)
                .sum(),
        }
    }

    fn dashboard_exercises(&self) -> Vec<String> {
        let mut names = self
            .filtered_trainings()
            .iter()
            .flat_map(|training| {
                training
                    .exercises
                    .iter()
                    .map(|exercise| exercise.name.clone())
            })
            .collect::<Vec<_>>();
        names.sort();
        names.dedup();
        names
    }

    fn ensure_dashboard_exercise(&mut self) {
        let names = self.dashboard_exercises();
        if names.is_empty() {
            self.selected_exercise = None;
        } else if !self
            .selected_exercise
            .as_ref()
            .is_some_and(|selected| names.contains(selected))
        {
            self.selected_exercise = names.first().cloned();
        }
    }

    fn cycle_dashboard_filter(&mut self, direction: i32) {
        match self.dashboard_filter {
            DashboardFilter::Year => self.cycle_year(direction),
            DashboardFilter::Month => self.cycle_month(direction),
            DashboardFilter::Exercise => self.cycle_exercise(direction),
        }
        self.ensure_dashboard_exercise();
        self.dashboard_scroll = 0;
    }

    fn move_dashboard_filter(&mut self, direction: i32) {
        self.dashboard_filter = match (self.dashboard_filter, direction) {
            (DashboardFilter::Year, 1) => DashboardFilter::Month,
            (DashboardFilter::Month, 1) => DashboardFilter::Exercise,
            (DashboardFilter::Exercise, 1) => DashboardFilter::Year,
            (DashboardFilter::Year, -1) => DashboardFilter::Exercise,
            (DashboardFilter::Month, -1) => DashboardFilter::Year,
            (DashboardFilter::Exercise, -1) => DashboardFilter::Month,
            _ => self.dashboard_filter,
        };
    }

    fn cycle_year(&mut self, direction: i32) {
        let Some(compiled) = self.compiled.as_ref() else {
            return;
        };
        let mut values = compiled
            .trainings
            .iter()
            .filter_map(|training| training.date.get(0..4).map(str::to_string))
            .collect::<Vec<_>>();
        values.sort();
        values.dedup();
        cycle_optional(&mut self.selected_year, &values, direction);
    }

    fn cycle_month(&mut self, direction: i32) {
        let values = (1..=12)
            .map(|month| format!("{month:02}"))
            .collect::<Vec<_>>();
        cycle_optional(&mut self.selected_month, &values, direction);
    }

    fn cycle_exercise(&mut self, direction: i32) {
        let values = self.dashboard_exercises();
        cycle_optional(&mut self.selected_exercise, &values, direction);
    }
}

struct SummaryView {
    total_trainings: usize,
    total_sets: usize,
    total_volume_kg: f32,
    total_cardio_km: f32,
}

#[derive(Debug, Clone)]
struct Completion {
    label: String,
    detail: String,
    insert_text: String,
    replace_start: usize,
    replace_end: usize,
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<(), String> {
    loop {
        terminal
            .draw(|frame| render(frame, app))
            .map_err(|err| err.to_string())?;
        if !event::poll(Duration::from_millis(200)).map_err(|err| err.to_string())? {
            continue;
        }
        match event::read().map_err(|err| err.to_string())? {
            Event::Key(key) => {
                if handle_key(app, key) {
                    break;
                }
            }
            Event::Mouse(mouse) => handle_mouse(app, mouse.kind),
            _ => {}
        }
    }
    Ok(())
}

fn handle_mouse(app: &mut App, kind: MouseEventKind) {
    if app.screen != Screen::Dashboard {
        return;
    }
    match kind {
        MouseEventKind::ScrollDown => app.dashboard_scroll = app.dashboard_scroll.saturating_add(3),
        MouseEventKind::ScrollUp => app.dashboard_scroll = app.dashboard_scroll.saturating_sub(3),
        _ => {}
    }
}

fn handle_dashboard_key(app: &mut App, key: KeyEvent) -> bool {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('c') => return true,
            KeyCode::Char('b') => app.screen = Screen::Editor,
            _ => {}
        }
        return false;
    }

    match key.code {
        KeyCode::Esc => app.screen = Screen::Editor,
        KeyCode::Up => app.dashboard_scroll = app.dashboard_scroll.saturating_sub(1),
        KeyCode::Down => app.dashboard_scroll = app.dashboard_scroll.saturating_add(1),
        KeyCode::PageUp => app.dashboard_scroll = app.dashboard_scroll.saturating_sub(8),
        KeyCode::PageDown => app.dashboard_scroll = app.dashboard_scroll.saturating_add(8),
        KeyCode::Left => app.move_dashboard_filter(-1),
        KeyCode::Right | KeyCode::Tab => app.move_dashboard_filter(1),
        KeyCode::Char('h') => app.move_dashboard_filter(-1),
        KeyCode::Char('l') => app.move_dashboard_filter(1),
        KeyCode::Char('j') => app.dashboard_scroll = app.dashboard_scroll.saturating_add(1),
        KeyCode::Char('k') => app.dashboard_scroll = app.dashboard_scroll.saturating_sub(1),
        KeyCode::Enter | KeyCode::Char(' ') => app.cycle_dashboard_filter(1),
        KeyCode::Backspace => {
            match app.dashboard_filter {
                DashboardFilter::Year => app.selected_year = None,
                DashboardFilter::Month => app.selected_month = None,
                DashboardFilter::Exercise => app.selected_exercise = None,
            }
            app.ensure_dashboard_exercise();
            app.dashboard_scroll = 0;
        }
        KeyCode::Char('n') => app.cycle_dashboard_filter(1),
        KeyCode::Char('p') => app.cycle_dashboard_filter(-1),
        _ => {}
    }
    false
}

fn handle_key(app: &mut App, key: KeyEvent) -> bool {
    if app.screen == Screen::Dashboard {
        return handle_dashboard_key(app, key);
    }

    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('c') => return true,
            KeyCode::Char('s') => app.save_all(),
            KeyCode::Char('e') => app.switch_buffer(),
            KeyCode::Char('n') => app.select_next_completion(),
            KeyCode::Char('p') => app.select_previous_completion(),
            KeyCode::Char('b') => {
                app.revalidate();
                app.screen = if app.screen == Screen::Dashboard {
                    Screen::Editor
                } else {
                    Screen::Dashboard
                };
            }
            _ => {}
        }
        return false;
    }

    match key.code {
        KeyCode::F(5) => app.compile(),
        KeyCode::Tab => app.accept_completion(),
        KeyCode::Esc => app.screen = Screen::Editor,
        KeyCode::Left => app.active_buffer_mut().move_left(),
        KeyCode::Right => app.active_buffer_mut().move_right(),
        KeyCode::Up => app.active_buffer_mut().move_up(),
        KeyCode::Down => app.active_buffer_mut().move_down(),
        KeyCode::Enter => {
            app.active_buffer_mut().newline();
            app.completion_index = 0;
            app.revalidate();
        }
        KeyCode::Backspace => {
            app.active_buffer_mut().backspace();
            app.completion_index = 0;
            app.revalidate();
        }
        KeyCode::Char(ch) => {
            app.active_buffer_mut().insert_char(ch);
            app.completion_index = 0;
            app.revalidate();
        }
        _ => {}
    }
    false
}

fn render(frame: &mut ratatui::Frame<'_>, app: &App) {
    let area = frame.area();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(area);

    render_header(frame, app, rows[0]);
    match app.screen {
        Screen::Editor => render_editor(frame, app, rows[1]),
        Screen::Dashboard => render_dashboard(frame, app, rows[1]),
    }
    render_status(frame, app, rows[2]);
}

fn render_header(frame: &mut ratatui::Frame<'_>, app: &App, area: Rect) {
    let title = Line::from(vec![
        Span::styled(
            " Fittrack TUI ",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        tab_span("training.fit", app.active == BufferKind::Training),
        Span::raw(" "),
        tab_span("exercises.txt", app.active == BufferKind::Exercises),
        Span::raw(" "),
        Span::styled(
            match app.screen {
                Screen::Editor => "Editor",
                Screen::Dashboard => "Dashboard",
            },
            Style::default().fg(Color::Yellow),
        ),
    ]);
    frame.render_widget(Paragraph::new(title).block(block("")), area);
}

const ASCII_BORDER: border::Set = border::Set {
    top_left: "+",
    top_right: "+",
    bottom_left: "+",
    bottom_right: "+",
    vertical_left: "|",
    vertical_right: "|",
    horizontal_top: "-",
    horizontal_bottom: "-",
};

fn block(title: &'static str) -> Block<'static> {
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_set(ASCII_BORDER)
}

fn tab_span(label: &'static str, active: bool) -> Span<'static> {
    if active {
        Span::styled(
            format!("[{label}]"),
            Style::default().fg(Color::Black).bg(Color::Green),
        )
    } else {
        Span::styled(format!(" {label} "), Style::default().fg(Color::Gray))
    }
}

fn render_editor(frame: &mut ratatui::Frame<'_>, app: &App, area: Rect) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(area);

    let buffer = app.active_buffer();
    let lines = buffer
        .lines
        .iter()
        .enumerate()
        .map(|(index, line)| syntax_line(line, index + 1, app, app.active))
        .collect::<Vec<_>>();
    let editor = Paragraph::new(lines)
        .block(block(buffer.title))
        .wrap(Wrap { trim: false });
    frame.render_widget(editor, columns[0]);

    let side_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(columns[1]);
    render_diagnostics(frame, app, side_rows[0]);
    render_completions(frame, app, side_rows[1]);

    let cursor_prefix =
        &buffer.current_line()[..buffer.cursor_col.min(buffer.current_line().len())];
    let x = columns[0].x + 1 + GUTTER_WIDTH as u16 + display_width(cursor_prefix) as u16;
    let y = columns[0].y + 1 + buffer.cursor_row as u16;
    if x < columns[0].right() && y < columns[0].bottom() {
        frame.set_cursor_position((x, y));
    }
}

const GUTTER_WIDTH: usize = 4;

fn syntax_line<'a>(line: &'a str, line_no: usize, app: &App, kind: BufferKind) -> Line<'a> {
    let has_error = app
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.line == Some(line_no));
    let number = Span::styled(
        format!("{line_no:>3} "),
        Style::default().fg(if has_error {
            Color::Red
        } else {
            Color::DarkGray
        }),
    );
    let trimmed = line.trim_start();
    let indent_len = line.len() - trimmed.len();
    let indent = Span::raw(&line[..indent_len]);
    let mut spans = vec![number, indent];

    if kind == BufferKind::Exercises {
        let style = if trimmed.starts_with('#') {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default().fg(Color::Cyan)
        };
        spans.push(Span::styled(trimmed, style));
        return Line::from(spans);
    }

    let keyword = trimmed.split_whitespace().next().unwrap_or_default();
    let keyword_style = match keyword {
        "training" => Some(Color::Green),
        "exercise" => Some(Color::Cyan),
        "set" => Some(Color::Yellow),
        "cardio" => Some(Color::Magenta),
        "note" => Some(Color::Blue),
        _ => None,
    };

    if let Some(color) = keyword_style {
        spans.push(Span::styled(
            keyword,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(&trimmed[keyword.len()..]));
    } else {
        spans.push(Span::styled(trimmed, Style::default().fg(Color::Red)));
    }
    Line::from(spans)
}

fn render_diagnostics(frame: &mut ratatui::Frame<'_>, app: &App, area: Rect) {
    let lines = if app.diagnostics.is_empty() {
        vec![Line::from(Span::styled(
            "No errors",
            Style::default().fg(Color::Green),
        ))]
    } else {
        app.diagnostics
            .iter()
            .map(|diagnostic| {
                let prefix = diagnostic
                    .line
                    .map(|line| format!("Line {line}: "))
                    .unwrap_or_default();
                Line::from(Span::styled(
                    format!("{prefix}{}", diagnostic.message),
                    Style::default().fg(Color::Red),
                ))
            })
            .collect()
    };
    frame.render_widget(
        Paragraph::new(lines)
            .block(block("Diagnostics"))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_completions(frame: &mut ratatui::Frame<'_>, app: &App, area: Rect) {
    let completions = app.completions();
    let lines = if completions.is_empty() {
        vec![Line::from(Span::styled(
            "No grammar choices for this cursor position",
            Style::default().fg(Color::DarkGray),
        ))]
    } else {
        completions
            .into_iter()
            .enumerate()
            .take(8)
            .map(|(index, item)| {
                let selected = index == app.completion_index;
                let style = if selected {
                    Style::default().fg(Color::Black).bg(Color::Cyan)
                } else {
                    Style::default().fg(Color::Cyan)
                };
                Line::from(vec![
                    Span::styled(if selected { "> " } else { "  " }, style),
                    Span::styled(item.label, style),
                    Span::styled(
                        format!("  {}", item.detail),
                        Style::default().fg(Color::DarkGray),
                    ),
                ])
            })
            .collect()
    };
    frame.render_widget(
        Paragraph::new(lines).block(block("Completions (Ctrl-N/P choose, Tab accept)")),
        area,
    );
}

fn render_dashboard(frame: &mut ratatui::Frame<'_>, app: &App, area: Rect) {
    let Some(compiled) = app.compiled.as_ref() else {
        frame.render_widget(
            Paragraph::new("Fix diagnostics to render the dashboard.").block(block("Dashboard")),
            area,
        );
        return;
    };

    let lines = dashboard_lines(app, compiled, area.width.saturating_sub(4) as usize);
    let visible_height = area.height.saturating_sub(2) as usize;
    let max_scroll = lines.len().saturating_sub(visible_height) as u16;
    let scroll = app.dashboard_scroll.min(max_scroll);
    frame.render_widget(
        Paragraph::new(lines)
            .block(block("Dashboard"))
            .scroll((scroll, 0))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_status(frame: &mut ratatui::Frame<'_>, app: &App, area: Rect) {
    let marker = if app.diagnostics.is_empty() {
        "[OK]"
    } else {
        "[ERR]"
    };
    let style = if app.diagnostics.is_empty() {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::Red)
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(marker, style),
            Span::raw(" "),
            Span::raw(app.status.as_str()),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_set(ASCII_BORDER),
        ),
        area,
    );
    if !app.diagnostics.is_empty() {
        let popup = centered_rect(60, 20, area);
        frame.render_widget(Clear, popup);
    }
}

fn dashboard_lines(app: &App, _compiled: &CompiledTraining, width: usize) -> Vec<Line<'static>> {
    let trainings = app.filtered_trainings();
    let summary = app.filtered_summary();
    let exercise_name = app
        .selected_exercise
        .clone()
        .or_else(|| app.dashboard_exercises().first().cloned())
        .unwrap_or_else(|| "No exercise".to_string());
    let chart_width = width.saturating_sub(24).clamp(12, 56);
    let mut lines = Vec::new();

    lines.push(Line::from(vec![
        Span::styled("Filters  ", Style::default().fg(Color::DarkGray)),
        filter_span(
            "Year",
            app.selected_year.as_deref().unwrap_or("All"),
            app.dashboard_filter == DashboardFilter::Year,
        ),
        Span::raw("  "),
        filter_span(
            "Month",
            app.selected_month
                .as_deref()
                .map(month_label)
                .unwrap_or("All"),
            app.dashboard_filter == DashboardFilter::Month,
        ),
        Span::raw("  "),
        filter_span(
            "Exercise",
            &exercise_name,
            app.dashboard_filter == DashboardFilter::Exercise,
        ),
    ]));
    lines.push(Line::from(Span::styled(
        "Left/Right choose filter | Space/Enter cycle | Backspace clear | Up/Down/Page scroll | mouse wheel scroll",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::raw(""));

    lines.push(section_title("Training summary"));
    lines.push(Line::from(vec![
        metric_card(
            "Sessions",
            summary.total_trainings.to_string(),
            Color::Green,
        ),
        Span::raw("  "),
        metric_card(
            "Strength Sets",
            summary.total_sets.to_string(),
            Color::Yellow,
        ),
    ]));
    lines.push(Line::from(vec![
        metric_card(
            "Total Volume",
            format!("{} kg", format_number(summary.total_volume_kg)),
            Color::Cyan,
        ),
        Span::raw("  "),
        metric_card(
            "Cardio Distance",
            format!("{} km", format_number(summary.total_cardio_km)),
            Color::Magenta,
        ),
    ]));
    lines.push(Line::raw(""));

    lines.push(section_title("Volume by session"));
    if trainings.is_empty() {
        lines.push(empty_line("No data for these filters"));
    } else {
        let max = trainings
            .iter()
            .map(|training| training_volume(training))
            .fold(1.0_f32, f32::max);
        for training in &trainings {
            let volume = training_volume(training);
            lines.push(Line::from(vec![
                Span::styled(
                    short_date(&training.date),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw(" "),
                Span::styled(
                    bar(volume, max, chart_width),
                    Style::default().fg(Color::Green),
                ),
                Span::raw(format!(" {} kg", format_number(volume))),
            ]));
        }
    }
    lines.push(Line::raw(""));

    lines.push(section_title("Exercise progression"));
    let points = trainings
        .iter()
        .filter_map(|training| {
            training
                .exercises
                .iter()
                .find(|exercise| exercise.name == exercise_name)
                .map(|exercise| {
                    (
                        short_date(&training.date),
                        best_estimated_max(exercise),
                        exercise_volume(exercise),
                    )
                })
        })
        .collect::<Vec<_>>();
    if points.is_empty() {
        lines.push(empty_line("No matching exercise data"));
    } else {
        let max_weight = points
            .iter()
            .map(|(_, weight, _)| *weight)
            .fold(1.0_f32, f32::max);
        let max_volume = points
            .iter()
            .map(|(_, _, volume)| *volume)
            .fold(1.0_f32, f32::max);
        lines.push(Line::from(vec![
            Span::styled("Estimated max", Style::default().fg(Color::Yellow)),
            Span::raw(format!(" · {exercise_name}")),
        ]));
        for (date, weight, _) in &points {
            lines.push(Line::from(vec![
                Span::styled(date.clone(), Style::default().fg(Color::DarkGray)),
                Span::raw(" "),
                Span::styled(
                    bar(*weight, max_weight, chart_width),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(format!(" {} kg", format_number(*weight))),
            ]));
        }
        lines.push(Line::from(Span::styled(
            "Volume",
            Style::default().fg(Color::Blue),
        )));
        for (date, _, volume) in &points {
            lines.push(Line::from(vec![
                Span::styled(date.clone(), Style::default().fg(Color::DarkGray)),
                Span::raw(" "),
                Span::styled(
                    bar(*volume, max_volume, chart_width),
                    Style::default().fg(Color::Blue),
                ),
                Span::raw(format!(" {} kg", format_number(*volume))),
            ]));
        }
    }
    lines.push(Line::raw(""));

    lines.push(section_title("Recent sessions"));
    if trainings.is_empty() {
        lines.push(empty_line("No sessions match these filters"));
    } else {
        for training in trainings.iter().rev() {
            lines.push(Line::from(Span::styled(
                format!("{} · {}", training.date, training.title),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )));
            let cardio = training
                .cardio
                .iter()
                .map(|item| {
                    format!(
                        "{} {}km in {}",
                        item.kind,
                        format_number(item.distance_km),
                        format_duration(item.duration_seconds)
                    )
                })
                .collect::<Vec<_>>()
                .join(" · ");
            if !cardio.is_empty() {
                lines.push(Line::from(Span::styled(
                    format!("  {cardio}"),
                    Style::default().fg(Color::DarkGray),
                )));
            }
            for exercise in &training.exercises {
                lines.push(Line::from(format!(
                    "  {}  {}",
                    exercise.name,
                    exercise
                        .sets
                        .iter()
                        .map(|set| format!("{} x {}kg", set.reps, format_number(set.weight_kg)))
                        .collect::<Vec<_>>()
                        .join(", ")
                )));
            }
            lines.push(Line::raw(""));
        }
    }
    lines
}

fn filter_span(label: &str, value: &str, focused: bool) -> Span<'static> {
    let text = format!("{label}: {value}");
    if focused {
        Span::styled(text, Style::default().fg(Color::Black).bg(Color::Cyan))
    } else {
        Span::styled(text, Style::default().fg(Color::Cyan))
    }
}

fn section_title(title: &'static str) -> Line<'static> {
    Line::from(Span::styled(
        title,
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    ))
}

fn metric_card(label: &str, value: String, color: Color) -> Span<'static> {
    Span::styled(
        format!("[ {label}: {value} ]"),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )
}

fn empty_line(message: &'static str) -> Line<'static> {
    Line::from(Span::styled(message, Style::default().fg(Color::DarkGray)))
}

fn month_label(value: &str) -> &str {
    match value {
        "01" => "January",
        "02" => "February",
        "03" => "March",
        "04" => "April",
        "05" => "May",
        "06" => "June",
        "07" => "July",
        "08" => "August",
        "09" => "September",
        "10" => "October",
        "11" => "November",
        "12" => "December",
        _ => value,
    }
}

fn cycle_optional(current: &mut Option<String>, values: &[String], direction: i32) {
    if values.is_empty() {
        *current = None;
        return;
    }

    let len = values.len() + 1;
    let current_index = current
        .as_ref()
        .and_then(|value| values.iter().position(|candidate| candidate == value))
        .map(|index| index + 1)
        .unwrap_or(0);
    let next_index = if direction >= 0 {
        (current_index + 1) % len
    } else {
        (current_index + len - 1) % len
    };
    *current = if next_index == 0 {
        None
    } else {
        values.get(next_index - 1).cloned()
    };
}

fn format_duration(seconds: u32) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

fn previous_boundary(input: &str, index: usize) -> usize {
    input[..index]
        .char_indices()
        .last()
        .map(|(idx, _)| idx)
        .unwrap_or(0)
}

fn next_boundary(input: &str, index: usize) -> usize {
    input[index..]
        .char_indices()
        .nth(1)
        .map(|(idx, _)| index + idx)
        .unwrap_or(input.len())
}

fn display_width(input: &str) -> usize {
    input.chars().count()
}

fn exercise_catalog_completions(buffer: &Buffer) -> Vec<Completion> {
    let line = buffer.current_line();
    let cursor = buffer.cursor_col.min(line.len());
    let prefix_start = line[..cursor]
        .rfind(|ch: char| ch == ',' || ch.is_whitespace())
        .map(|index| index + 1)
        .unwrap_or(0);
    let prefix = &line[prefix_start..cursor];
    completion_items(
        prefix,
        prefix_start,
        cursor,
        [
            ("Back Squat", "exercise catalog entry"),
            ("Bench Press", "exercise catalog entry"),
            ("Deadlift", "exercise catalog entry"),
            ("Overhead Press", "exercise catalog entry"),
            ("Pull Up", "exercise catalog entry"),
            ("Romanian Deadlift", "exercise catalog entry"),
        ],
    )
}

fn training_completions(buffer: &Buffer, exercise_source: &str) -> Vec<Completion> {
    let line = buffer.current_line();
    let cursor = buffer.cursor_col.min(line.len());
    let before = &line[..cursor];
    let line_start = before.len() - before.trim_start().len();
    let trimmed_before = &before[line_start..];
    if trimmed_before.starts_with("exercise ") {
        return exercise_name_completions(line, cursor, exercise_source);
    }
    if trimmed_before.starts_with("set ") {
        return statement_body_completions(
            line,
            cursor,
            "set",
            [
                ("5 x 60kg @8", "strength set with RPE"),
                ("8 x 40kg", "strength set"),
                ("10 x 20kg @7", "volume set with RPE"),
            ],
        );
    }
    if trimmed_before.starts_with("cardio ") {
        return statement_body_completions(
            line,
            cursor,
            "cardio",
            [
                ("run 5km 25:00", "cardio run"),
                ("bike 10km 20:00", "cardio bike"),
                ("row 3km 12:00", "cardio row"),
            ],
        );
    }
    if trimmed_before.starts_with("training ") {
        return statement_body_completions(
            line,
            cursor,
            "training",
            [
                ("2026-05-11 \"Session\"", "training header"),
                ("2026-05-11 \"Push\"", "training header"),
                ("2026-05-11 \"Pull\"", "training header"),
            ],
        );
    }
    if trimmed_before.starts_with("note ") {
        return statement_body_completions(
            line,
            cursor,
            "note",
            [
                ("\"Felt strong.\"", "quoted note"),
                ("\"Keep the same warmup.\"", "quoted note"),
            ],
        );
    }

    let keyword_end = line_start + trimmed_before.len();
    completion_items(
        trimmed_before,
        line_start,
        keyword_end,
        [
            ("training 2026-05-11 \"Session\"", "start a training block"),
            ("exercise \"\"", "start an exercise"),
            ("set 5 x 60kg @8", "add a strength set"),
            ("cardio run 5km 25:00", "add cardio work"),
            ("note \"\"", "add a note"),
        ],
    )
}

fn exercise_name_completions(line: &str, cursor: usize, exercise_source: &str) -> Vec<Completion> {
    let Some(keyword_index) = line.find("exercise") else {
        return Vec::new();
    };
    let body_start = keyword_index + "exercise".len();
    if cursor < body_start {
        return Vec::new();
    }
    let after_keyword = &line[body_start..cursor];
    let leading_space = after_keyword.len() - after_keyword.trim_start().len();
    let quote_start = body_start + leading_space;
    let starts_with_quote = line[quote_start..cursor].starts_with('"');
    let replace_start = if starts_with_quote {
        quote_start
    } else {
        body_start + leading_space
    };
    let raw_prefix = &line[replace_start..cursor];
    let prefix = raw_prefix.trim_start_matches('"');
    let names = exercise_names(exercise_source)
        .into_iter()
        .map(|name| {
            let label = name.clone();
            (format!("\"{name}\""), label)
        })
        .collect::<Vec<_>>();
    names
        .into_iter()
        .filter(|(_, label)| matches_prefix(label, prefix))
        .map(|(insert_text, label)| Completion {
            label,
            detail: "catalog exercise".to_string(),
            insert_text,
            replace_start,
            replace_end: cursor,
        })
        .collect()
}

fn statement_body_completions<const N: usize>(
    line: &str,
    cursor: usize,
    keyword: &str,
    candidates: [(&'static str, &'static str); N],
) -> Vec<Completion> {
    let Some(keyword_index) = line.find(keyword) else {
        return Vec::new();
    };
    let body_start = keyword_index + keyword.len();
    if cursor < body_start {
        return Vec::new();
    }
    let after_keyword = &line[body_start..cursor];
    let leading_space = after_keyword.len() - after_keyword.trim_start().len();
    let replace_start = body_start + leading_space;
    let prefix = &line[replace_start..cursor];
    completion_items(prefix, replace_start, cursor, candidates)
}

fn completion_items<const N: usize>(
    prefix: &str,
    replace_start: usize,
    replace_end: usize,
    candidates: [(&'static str, &'static str); N],
) -> Vec<Completion> {
    candidates
        .into_iter()
        .filter(|(candidate, _)| matches_prefix(candidate, prefix))
        .map(|(candidate, detail)| Completion {
            label: candidate.to_string(),
            detail: detail.to_string(),
            insert_text: candidate.to_string(),
            replace_start,
            replace_end,
        })
        .collect()
}

fn matches_prefix(candidate: &str, prefix: &str) -> bool {
    let prefix = prefix.trim().trim_matches('"').to_ascii_lowercase();
    prefix.is_empty() || candidate.to_ascii_lowercase().contains(&prefix)
}

fn parse_line_no(message: &str) -> Option<usize> {
    message
        .strip_prefix("Line ")
        .and_then(|rest| rest.split_once(':'))
        .and_then(|(line, _)| line.parse().ok())
        .or_else(|| {
            message
                .strip_prefix("Exercise catalog line ")
                .and_then(|rest| rest.split_once(':'))
                .and_then(|(line, _)| line.parse().ok())
        })
}

fn exercise_names(source: &str) -> Vec<String> {
    ExerciseCatalog::parse(source)
        .map(|catalog| catalog.values().map(str::to_string).collect())
        .unwrap_or_default()
}

fn training_volume(training: &Training) -> f32 {
    training.exercises.iter().map(exercise_volume).sum()
}

fn exercise_volume(exercise: &Exercise) -> f32 {
    exercise
        .sets
        .iter()
        .map(|set| set.reps as f32 * set.weight_kg)
        .sum()
}

fn best_estimated_max(exercise: &Exercise) -> f32 {
    exercise
        .sets
        .iter()
        .map(|set| set.weight_kg * (1.0 + set.reps as f32 / 30.0))
        .fold(0.0, f32::max)
}

fn bar(value: f32, max: f32, width: usize) -> String {
    let filled = ((value / max.max(1.0)) * width as f32).round() as usize;
    format!(
        "{}{}",
        "#".repeat(filled),
        "-".repeat(width.saturating_sub(filled))
    )
}

fn short_date(date: &str) -> String {
    date.get(5..).unwrap_or(date).to_string()
}

fn format_number(value: f32) -> String {
    let value = if value.abs() < 0.05 { 0.0 } else { value };
    let rounded = (value * 10.0).round() / 10.0;
    if rounded.fract() == 0.0 {
        format!("{rounded:.0}")
    } else {
        format!("{rounded:.1}")
    }
}
