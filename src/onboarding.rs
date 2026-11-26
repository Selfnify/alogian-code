use std::{io, path::PathBuf};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    DefaultTerminal, Frame,
};

use crate::background::{colors::active as theme, ui};

const GOALS: [&str; 5] = [
    "Build software",
    "Write and edit documents",
    "Analyze data",
    "Learn and explore",
    "Something else",
];
const THEME_STEP: usize = 3;
const LAST_STEP: usize = 4;

struct State {
    step: usize,
    name: String,
    goal: usize,
    theme: usize,
    original_theme: usize,
}

fn profile_path() -> Option<PathBuf> {
    let base = match std::env::var_os("LOCALAPPDATA") {
        Some(dir) => PathBuf::from(dir),
        None => {
            let home = PathBuf::from(std::env::var_os("HOME")?);
            if cfg!(target_os = "macos") {
                home.join("Library/Application Support")
            } else {
                home
            }
        }
    };
    Some(base.join("LumeCode").join("profile.txt"))
}

fn load() -> State {
    let mut state = State {
        step: 0,
        name: String::new(),
        goal: 0,
        theme: theme::index(),
        original_theme: theme::index(),
    };
    if let Some(text) = profile_path().and_then(|path| std::fs::read_to_string(path).ok()) {
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key {
                "step" => state.step = value.parse().unwrap_or(0),
                "name" => state.name = value.to_string(),
                "goal" => state.goal = value.parse().unwrap_or(0),
                _ => {}
            }
        }
    }
    state
}

fn save(state: &State) {
    if let Some(path) = profile_path() {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(
            path,
            format!("step={}\nname={}\ngoal={}\n", state.step, state.name, state.goal),
        );
    }
}

pub fn run(terminal: &mut DefaultTerminal) -> io::Result<()> {
    let mut state = load();
    if state.step > LAST_STEP {
        return Ok(());
    }
    loop {
        terminal.draw(|frame| draw(frame, &state))?;
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            theme::set_index(state.original_theme);
            ratatui::restore();
            std::process::exit(0);
        }
        match key.code {
            KeyCode::Esc => {
                if state.step == THEME_STEP {
                    theme::set_index(state.original_theme);
                    state.theme = state.original_theme;
                }
                state.step = state.step.saturating_sub(1);
                save(&state);
            }
            KeyCode::Enter => {
                if state.step == THEME_STEP {
                    theme::save_current();
                }
                state.step += 1;
                save(&state);
                if state.step > LAST_STEP {
                    return Ok(());
                }
            }
            _ => handle_input(&mut state, key),
        }
    }
}

fn handle_input(state: &mut State, key: KeyEvent) {
    match state.step {
        1 => match key.code {
            KeyCode::Char(c) if !c.is_control() && state.name.chars().count() < 40 => {
                state.name.push(c)
            }
            KeyCode::Backspace => {
                state.name.pop();
            }
            _ => {}
        },
        2 => match key.code {
            KeyCode::Up => state.goal = state.goal.saturating_sub(1),
            KeyCode::Down => state.goal = (state.goal + 1).min(GOALS.len() - 1),
            KeyCode::Char(c) => {
                if let Some(n) = c.to_digit(10) {
                    if (1..=GOALS.len() as u32).contains(&n) {
                        state.goal = n as usize - 1;
                    }
                }
            }
            _ => {}
        },
        THEME_STEP => match key.code {
            KeyCode::Up => {
                state.theme = state.theme.saturating_sub(1);
                theme::set_index(state.theme);
            }
            KeyCode::Down => {
                state.theme = (state.theme + 1).min(theme::count() - 1);
                theme::set_index(state.theme);
            }
            _ => {}
        },
        _ => {}
    }
}

fn draw(frame: &mut Frame, state: &State) {
    ui::draw(frame);
    let area = frame.area();
    let w = area.width.saturating_sub(4).min(80);
    let h = area.height.saturating_sub(2).min(24);
    let outer = Rect::new(
        area.x + area.width.saturating_sub(w) / 2,
        area.y + area.height.saturating_sub(h) / 2,
        w,
        h,
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::accent()))
        .style(Style::default().bg(theme::bg()));
    let inner = block.inner(outer);
    frame.render_widget(block, outer);
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .margin(1)
    .split(inner);
    frame.render_widget(
        Paragraph::new(format!(
            "Alogian Code setup · step {} of {}",
            state.step + 1,
            LAST_STEP + 1
        ))
        .style(
            Style::default()
                .fg(theme::accent())
                .add_modifier(Modifier::BOLD),
        ),
        chunks[0],
    );
    frame.render_widget(
        Paragraph::new(body(state))
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(theme::text())),
        chunks[1],
    );
    frame.render_widget(
        Paragraph::new(hint(state.step)).style(Style::default().fg(theme::text())),
        chunks[2],
    );
}

fn body(state: &State) -> Vec<Line<'static>> {
    match state.step {
        0 => vec![
            Line::raw("Welcome to Alogian Code."),
            Line::raw(""),
            Line::raw("A few quick questions, then you can start working on your project."),
        ],
        1 => vec![
            Line::raw("What should Alogian call you?"),
            Line::raw(""),
            Line::raw(format!("> {}", state.name)),
            Line::raw(""),
            Line::raw("Optional. Leave it empty to skip."),
        ],
        2 => {
            let mut lines = vec![
                Line::raw("What will you mostly use Alogian for?"),
                Line::raw(""),
            ];
            for (i, goal) in GOALS.iter().enumerate() {
                let marker = if i == state.goal { ">" } else { " " };
                lines.push(Line::raw(format!("{marker} {}  {goal}", i + 1)));
            }
            lines
        }
        THEME_STEP => {
            let mut lines = vec![
                Line::raw("Pick a theme. It previews as you move."),
                Line::raw(""),
            ];
            for (i, name) in theme::names().iter().enumerate() {
                let marker = if i == state.theme { ">" } else { " " };
                lines.push(Line::raw(format!(
                    "{marker} {name:<18} {}",
                    theme::description(i)
                )));
            }
            lines
        }
        _ => vec![
            Line::raw("All set."),
            Line::raw(""),
            Line::raw(format!(
                "Name: {}",
                if state.name.is_empty() {
                    "not set"
                } else {
                    &state.name
                }
            )),
            Line::raw(format!("Goal: {}", GOALS[state.goal])),
            Line::raw(format!("Theme: {}", theme::current_name())),
        ],
    }
}

fn hint(step: usize) -> &'static str {
    match step {
        0 => "Enter continue",
        1 => "Type your name · Enter continue · Esc back",
        2 => "↑↓ or 1-5 to choose · Enter continue · Esc back",
        THEME_STEP => "↑↓ to preview · Enter save · Esc back",
        _ => "Enter to start · Esc back",
    }
}