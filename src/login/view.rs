use super::{Model, Phase};
use crate::background::{colors::active as theme, ui};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

#[derive(Default)]
pub struct Buttons {
    pub login: Rect,
    pub copy: Option<Rect>,
    pub cancel: Rect,
}

const LOGO_BITMAP: [&str; 10] = [
    "1111111111111",
    "1000000000001",
    "1001100011001",
    "1001100011001",
    "1000000000001",
    "1010000000101",
    "1001111111001",
    "1000000000001",
    "1111111111111",
    "0000000000000",
];

fn logo_rows() -> Vec<String> {
    let on = |py: usize, px: usize| LOGO_BITMAP[py].as_bytes()[px] == b'1';
    (0..5)
        .map(|r| {
            (0..13)
                .map(|c| match (on(r * 2, c), on(r * 2 + 1, c)) {
                    (true, true) => '█',
                    (true, false) => '▀',
                    (false, true) => '▄',
                    _ => ' ',
                })
                .collect()
        })
        .collect()
}

fn option(number: usize, text: &str, description: &str, selected: bool, error: bool) -> Vec<Line<'static>> {
    let accent = theme::accent();
    let text_color = theme::text();
    let description_color = if error { Color::Red } else if selected { accent } else { text_color };
    let heading = if selected {
        Line::from(vec![
            Span::styled(
                format!("> {number}. "),
                Style::default().fg(accent).add_modifier(Modifier::DIM),
            ),
            Span::styled(text.to_string(), Style::default().fg(accent)),
        ])
    } else {
        Line::raw(format!("  {number}. {text}"))
    };
    let detail = Line::styled(
        format!("     {description}"),
        Style::default()
            .fg(description_color)
            .add_modifier(Modifier::DIM),
    );
    vec![heading, detail, Line::raw("")]
}

pub fn draw(frame: &mut Frame, model: &Model) -> Buttons {
    ui::draw(frame);
    let full = frame.area();
    let area = Rect::new(
        full.x + 2,
        full.y + 1,
        full.width.saturating_sub(4),
        full.height.saturating_sub(2),
    );
    let accent = theme::accent();
    let mut lines: Vec<Line<'static>> = logo_rows()
        .into_iter()
        .map(|row| Line::styled(format!("  {row}"), Style::default().fg(accent)))
        .collect();
    lines.push(Line::raw(""));
    lines.push(Line::raw(format!(
        "  Welcome to Alogian Code (v{}).",
        env!("CARGO_PKG_VERSION")
    )));
    lines.push(Line::raw(
        "  Sign in with your Alogian account to start working on your project.",
    ));
    lines.push(Line::raw(""));

    let error = matches!(model.phase, Phase::Error);
    let label = match model.phase {
        Phase::Waiting => "Open browser",
        Phase::Starting => "Creating sign-in link…",
        Phase::Error => "Try again",
        Phase::Connected => "Signed in",
        Phase::Idle => "Sign in with browser",
    };
    let body_start = lines.len();
    lines.extend(option(1, label, &model.status, model.selected == 0, error));

    let link = model.link.is_some();
    let copy_index = body_start + 3;
    if link {
        lines.extend(option(2, "Copy URL", "Press C to copy the sign-in URL", model.selected == 1, false));
    }
    let cancel_index = if link { body_start + 6 } else { body_start + 3 };
    let cancel_number = if link { 3 } else { 2 };
    lines.extend(option(cancel_number, "Cancel", "Press Esc to cancel", model.selected == cancel_number - 1, false));

    if let Some(link) = &model.link {
        lines.extend([
            Line::styled(
                format!("  Confirmation code: {}", link.code),
                Style::default().fg(accent),
            ),
            Line::raw("  Check this code in your browser before approving."),
            Line::raw(""),
            Line::raw("  Public sign-in token"),
            Line::raw(format!("  {}", link.token)),
            Line::raw(""),
            Line::raw("  If the browser did not open, copy this URL:"),
            Line::styled(format!("  {}", link.url), Style::default().fg(accent)),
            Line::raw(""),
        ]);
        let remaining = link.expires_in.saturating_sub(
            model
                .link_started
                .map(|t| t.elapsed().as_secs())
                .unwrap_or(0),
        );
        lines.push(Line::raw(format!(
            "  Link expires in {}:{:02}",
            remaining / 60,
            remaining % 60
        )));
    } else {
        lines.push(Line::raw(
            "  Your browser handles sign-in. Your session is stored securely on this PC.",
        ));
    }

    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::styled("  Press ", Style::default().add_modifier(Modifier::DIM)),
        Span::styled("Enter", Style::default().fg(accent)),
        Span::styled(" to continue", Style::default().add_modifier(Modifier::DIM)),
    ]));
    lines.push(Line::styled(
        "  Esc to cancel · C copy URL · Page Up/Down to scroll",
        Style::default().add_modifier(Modifier::DIM),
    ));

    let logo_len = 6;
    let block_width = lines
        .iter()
        .skip(logo_len)
        .map(|line| line.width())
        .max()
        .unwrap_or(0);
    let pad = " ".repeat((area.width as usize).saturating_sub(block_width) / 2);
    for line in lines.iter_mut().skip(logo_len) {
        line.spans.insert(0, Span::raw(pad.clone()));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((model.scroll, 0))
            .style(Style::default().fg(theme::text()).bg(theme::bg())),
        area,
    );

    let row = |index: usize| {
        let y = area.y as i32 + index as i32 - model.scroll as i32;
        Rect::new(area.x, y.max(0) as u16, area.width, 1)
    };
    Buttons {
        login: row(body_start),
        copy: if link { Some(row(copy_index)) } else { None },
        cancel: row(cancel_index),
    }
}