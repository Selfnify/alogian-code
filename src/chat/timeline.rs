use super::tasker::Task;
use crate::background::colors::active as theme;
const DIFF_ADD_BG: Color = Color::Rgb(33, 58, 43);
#[cfg(test)]
mod tests;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
#[derive(Default)]
pub struct Timeline {
    pub groups: Vec<Group>,
    pub current: Option<usize>,
    pub full: bool,
}
pub struct Group {
    pub tasks: Vec<Task>,
    pub open: bool,
    pub started: std::time::Instant,
    pub finished: Option<u64>,
    pub settled: bool,
    pub restored: bool,
}
fn command_lines(detail: &str) -> Vec<String> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(detail) else {
        return detail.lines().map(str::to_owned).collect();
    };
    if v.get("stdout").is_none() && v.get("error").is_none() {
        return detail.lines().map(str::to_owned).collect();
    }
    let mut out: Vec<String> = v["stdout"]
        .as_str()
        .unwrap_or("")
        .lines()
        .map(str::to_owned)
        .collect();
    out.extend(v["stderr"].as_str().unwrap_or("").lines().map(str::to_owned));
    if let Some(code) = v["exit_code"].as_i64().filter(|c| *c != 0) {
        out.push(format!("exit {code}"));
    }
    if let Some(error) = v["error"].as_str() {
        out.push(error.to_owned());
    }
    out
}
fn readable_on_dark(color: Color) -> Color {
    match color {
        Color::Rgb(r, g, b) => {
            let lum = (r as u32 * 299 + g as u32 * 587 + b as u32 * 114) / 1000;
            if lum < 150 {
                Color::Rgb(
                    r.saturating_add(120),
                    g.saturating_add(120),
                    b.saturating_add(120),
                )
            } else {
                color
            }
        }
        other => other,
    }
}

impl Timeline {
    pub fn latest(&mut self) {
        if let Some(g) = self.groups.last_mut() {
            g.open = !g.open;
        }
    }
    pub fn sync(&mut self, tasks: &[Task]) -> Option<usize> {
        let unseen = tasks
            .iter()
            .filter(|t| {
                !self
                    .groups
                    .iter()
                    .any(|g| g.tasks.iter().any(|v| v.id == t.id))
            })
            .cloned()
            .collect::<Vec<_>>();
        let mut created = None;
        if !unseen.is_empty() {
            let index = match self.current {
                Some(i) => i,
                None => {
                    let i = self.groups.len();
                    self.groups.push(Group {
                        tasks: vec![],
                        open: false,
                        started: std::time::Instant::now(),
                        finished: None,
                        settled: false,
                        restored: false,
                    });
                    self.current = Some(i);
                    created = Some(i);
                    i
                }
            };
            self.groups[index].tasks.extend(unseen);
        }
        for g in &mut self.groups {
            if g.settled {
                continue;
            }
            for task in &mut g.tasks {
                if let Some(v) = tasks.iter().find(|v| v.id == task.id) {
                    *task = v.clone();
                }
            }
            if g.tasks.iter().any(|t| !t.done) {
                g.finished = None;
            }
            if g.tasks.iter().all(|t| t.done) && g.finished.is_none() {
                g.finished = Some(g.started.elapsed().as_secs());
            }
        }
        created
    }
    pub fn render(&self, index: usize, lines: &mut Vec<Line<'static>>) {
        let Some(group) = self.groups.get(index) else {
            return;
        };
        let phase = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            / 400) as usize;
        for task in &group.tasks {
            let is_write = super::tasker::is_write_tool(&task.name);
            if is_write && !task.done {
                continue;
            }
            let diff = is_write && task.done && !task.failed && !task.detail.is_empty();
            let color = if !task.done {
                if phase % 2 == 0 {
                    Color::White
                } else {
                    Color::DarkGray
                }
            } else if task.failed {
                Color::Red
            } else {
                Color::Green
            };
            if diff {
                let path = task.label.strip_prefix("Created ").unwrap_or(&task.label);
                let all: Vec<&str> = task.detail.lines().collect();
                let total = all.len();
                let width = total.to_string().len();
                lines.push(Line::from(vec![
                    Span::styled("• ", Style::default().add_modifier(Modifier::DIM)),
                    Span::styled("Added", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(format!(" {path} ")),
                    Span::styled(format!("+{total}"), Style::default().fg(Color::Green)),
                ]));
                let lang = path.rsplit('.').next().unwrap_or("txt");
                let tokens = super::markdown::highlight(&task.detail, lang);
                let keep = if self.full { total } else { total.min(3) };
                for (i, text) in all.iter().take(keep).enumerate() {
                    let mut spans = vec![
                        Span::styled(
                            format!("{:>width$} ", i + 1),
                            Style::default()
                                .fg(theme::text())
                                .add_modifier(Modifier::DIM),
                        ),
                        Span::styled(
                            "+ ",
                            Style::default().fg(Color::Green).bg(DIFF_ADD_BG),
                        ),
                    ];
                    match tokens.get(i) {
                        Some(parts) => {
                            let mut budget = 160usize;
                            for (part, color) in parts {
                                let clean_part: String =
                                    crate::clean(part).chars().take(budget).collect();
                                budget = budget.saturating_sub(clean_part.chars().count());
                                if clean_part.is_empty() {
                                    continue;
                                }
                                spans.push(Span::styled(
                                    clean_part,
                                    Style::default()
                                        .fg(readable_on_dark(*color))
                                        .bg(DIFF_ADD_BG),
                                ));
                            }
                        }
                        None => {
                            let plain: String =
                                crate::clean(text).chars().take(160).collect();
                            spans.push(Span::styled(
                                plain,
                                Style::default().fg(Color::Green).bg(DIFF_ADD_BG),
                            ));
                        }
                    }
                    lines.push(Line::from(spans).style(Style::default().bg(DIFF_ADD_BG)));
                }
                if total > keep {
                    lines.push(Line::styled(
                        format!("  … +{} lines (Ctrl-O)", total - keep),
                        Style::default()
                            .fg(theme::text())
                            .add_modifier(Modifier::DIM),
                    ));
                }
                lines.push(Line::raw(""));
                continue;
            }
            lines.push(Line::from(vec![
                Span::styled("● ", Style::default().fg(color)),
                Span::raw(task.label.clone()),
            ]));
            let detail: Vec<String> = if task.name == "get_datetime" {
                vec![]
            } else if task.name == "run_command" {
                command_lines(&task.detail)
            } else {
                task.detail.lines().map(str::to_owned).collect()
            };
            let keep = if self.full { 30 } else { 3 };
            let visible = &detail[detail.len().saturating_sub(keep)..];
            let is_diff = detail.iter().any(|l| l.starts_with("@@"));
            if visible.is_empty() {
                lines.push(Line::styled(
                    "  └ (no output)",
                    Style::default().fg(theme::text()),
                ));
            }
            for (i, text) in visible.iter().enumerate() {
                let branch = if i == 0 { "└ " } else { "  " };
                let clean_text = crate::clean(text.as_str()).chars().take(160).collect::<String>();
                let color = if is_diff {
                    if clean_text.starts_with("@@") {
                        theme::accent()
                    } else if clean_text.starts_with('+') && !clean_text.starts_with("+++") {
                        Color::Green
                    } else if clean_text.starts_with('-') && !clean_text.starts_with("---") {
                        Color::Red
                    } else {
                        theme::text()
                    }
                } else {
                    theme::text()
                };
                lines.push(Line::styled(
                    format!("  {branch}{clean_text}"),
                    Style::default().fg(color),
                ));
            }
            lines.push(Line::raw(""));
        }
        lines.push(Line::raw(""));
    }
}