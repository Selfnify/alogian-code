use ratatui::style::Color;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

use super::{arctic, cream_terracotta, forest, midnight, obsidian, sunset};

#[derive(Clone, Copy)]
pub struct Palette {
    pub bg: Color,
    pub accent: Color,
    pub text: Color,
}

impl Palette {
    pub fn surface(&self) -> Color {
        blend(self.bg, self.text, 12)
    }
    pub fn muted(&self) -> Color {
        blend(self.bg, self.text, 60)
    }
    pub fn diff_add(&self) -> Color {
        blend(self.bg, Color::Rgb(60, 200, 110), 18)
    }
}

pub const THEMES: [(&str, Palette); 6] = [
    (
        "cream-terracotta",
        Palette {
            bg: cream_terracotta::BG,
            accent: cream_terracotta::ACCENT,
            text: cream_terracotta::TEXT,
        },
    ),
    ("midnight", midnight::PALETTE),
    ("obsidian", obsidian::PALETTE),
    ("forest", forest::PALETTE),
    ("arctic", arctic::PALETTE),
    ("sunset", sunset::PALETTE),
];

pub const DESCRIPTIONS: [&str; 6] = [
    "Warm cream with terracotta accents",
    "Dark navy with blue accents",
    "Deep black with purple accents",
    "Dark green with mint accents",
    "Cool white with cyan accents",
    "Warm peach with orange accents",
];

static CURRENT: AtomicUsize = AtomicUsize::new(1);
static ALL: OnceLock<Vec<(String, Palette)>> = OnceLock::new();

fn all() -> &'static [(String, Palette)] {
    ALL.get_or_init(|| {
        let mut list: Vec<(String, Palette)> = THEMES
            .iter()
            .map(|(name, palette)| (name.to_string(), *palette))
            .collect();
        for (name, palette) in custom_themes() {
            if !list.iter().any(|(existing, _)| *existing == name) {
                list.push((name, palette));
            }
        }
        list
    })
}

pub fn index() -> usize {
    CURRENT.load(Ordering::Relaxed).min(all().len() - 1)
}

pub fn set_index(index: usize) {
    CURRENT.store(index.min(all().len() - 1), Ordering::Relaxed);
}

pub fn description(index: usize) -> &'static str {
    DESCRIPTIONS.get(index).copied().unwrap_or("Custom theme")
}

pub fn count() -> usize {
    all().len()
}

pub fn palette() -> Palette {
    all()[index()].1
}

pub fn bg() -> Color {
    palette().bg
}

pub fn accent() -> Color {
    palette().accent
}

pub fn text() -> Color {
    palette().text
}

pub fn names() -> Vec<&'static str> {
    all().iter().map(|(name, _)| name.as_str()).collect()
}

pub fn current_name() -> &'static str {
    all()[index()].0.as_str()
}

pub fn select(name: &str) -> bool {
    match all().iter().position(|(n, _)| n == name) {
        Some(index) => {
            CURRENT.store(index, Ordering::Relaxed);
            true
        }
        None => false,
    }
}

fn data_dir() -> Option<std::path::PathBuf> {
    let base = match std::env::var_os("LOCALAPPDATA") {
        Some(dir) => std::path::PathBuf::from(dir),
        None => {
            let home = std::path::PathBuf::from(std::env::var_os("HOME")?);
            if cfg!(target_os = "macos") {
                home.join("Library/Application Support")
            } else {
                home
            }
        }
    };
    Some(base.join("LumeCode"))
}

fn settings_path() -> Option<std::path::PathBuf> {
    Some(data_dir()?.join("theme.txt"))
}

fn custom_themes() -> Vec<(String, Palette)> {
    let Some(dir) = data_dir().map(|dir| dir.join("themes")) else {
        return vec![];
    };
    let Ok(entries) = std::fs::read_dir(dir) else {
        return vec![];
    };
    let mut out = vec![];
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("theme") {
            continue;
        }
        let Some(name) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        if let Some(palette) = parse_palette(&text) {
            out.push((name.to_string(), ensure_contrast(palette)));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn parse_palette(text: &str) -> Option<Palette> {
    let (mut bg, mut accent, mut fg) = (None, None, None);
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let color = parse_hex(value.trim());
        match key.trim() {
            "bg" => bg = color,
            "accent" => accent = color,
            "text" => fg = color,
            _ => {}
        }
    }
    Some(Palette {
        bg: bg?,
        accent: accent?,
        text: fg?,
    })
}

fn parse_hex(value: &str) -> Option<Color> {
    let hex = value.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color::Rgb(red, green, blue))
}

fn luminance(color: Color) -> f64 {
    let Color::Rgb(r, g, b) = color else {
        return 0.5;
    };
    let linear = |c: u8| {
        let c = c as f64 / 255.0;
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b)
}

fn contrast(a: Color, b: Color) -> f64 {
    let (x, y) = (luminance(a), luminance(b));
    let (light, dark) = if x > y { (x, y) } else { (y, x) };
    (light + 0.05) / (dark + 0.05)
}

pub fn ensure_contrast(palette: Palette) -> Palette {
    let mut out = palette;
    if contrast(out.text, out.bg) < 4.5 {
        out.text = if luminance(out.bg) > 0.5 {
            Color::Rgb(20, 20, 20)
        } else {
            Color::Rgb(240, 240, 240)
        };
    }
    if contrast(out.accent, out.bg) < 3.0 {
        out.accent = out.text;
    }
    out
}

fn blend(base: Color, ink: Color, ink_percent: u16) -> Color {
    match (base, ink) {
        (Color::Rgb(br, bg, bb), Color::Rgb(ir, ig, ib)) => {
            let mix = |b: u8, i: u8| {
                ((b as u16 * (100 - ink_percent) + i as u16 * ink_percent) / 100) as u8
            };
            Color::Rgb(mix(br, ir), mix(bg, ig), mix(bb, ib))
        }
        _ => base,
    }
}

fn default_index() -> usize {
    let dark = std::env::var("COLORFGBG")
        .ok()
        .and_then(|value| value.rsplit(';').next().and_then(|bg| bg.parse::<u8>().ok()))
        .is_some_and(|bg| bg <= 6 || bg == 8);
    if dark {
        1
    } else {
        0
    }
}

pub fn load_saved() {
    let saved = settings_path().and_then(|path| std::fs::read_to_string(path).ok());
    let selected = saved.is_some_and(|name| select(name.trim()));
    if !selected {
        set_index(default_index());
    }
}

pub fn save_current() {
    if let Some(path) = settings_path() {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(path, current_name());
    }
}