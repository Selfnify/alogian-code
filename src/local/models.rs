use serde_json::Value;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

pub struct Status {
    pub ollama_running: bool,
    pub models: Vec<(String, f64)>,
    pub llama_running: bool,
}

fn get(port: u16, path: &str) -> Option<String> {
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().ok()?;
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_millis(400)).ok()?;
    stream.set_read_timeout(Some(Duration::from_secs(2))).ok()?;
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n"
    )
    .ok()?;
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).ok()?;
    let text = String::from_utf8_lossy(&raw).to_string();
    let (head, body) = text.split_once("\r\n\r\n")?;
    if !head.starts_with("HTTP/1.1 200") && !head.starts_with("HTTP/1.0 200") {
        return None;
    }
    if head.to_ascii_lowercase().contains("transfer-encoding: chunked") {
        return Some(decode_chunked(body));
    }
    Some(body.to_string())
}

fn decode_chunked(body: &str) -> String {
    let mut out = String::new();
    let mut rest = body;
    loop {
        let Some((size_line, after)) = rest.split_once("\r\n") else {
            break;
        };
        let size = usize::from_str_radix(size_line.trim(), 16).unwrap_or(0);
        if size == 0 || after.len() < size {
            break;
        }
        out.push_str(&after[..size]);
        rest = after.get(size + 2..).unwrap_or("");
    }
    out
}

pub fn status() -> Status {
    let models = get(11434, "/api/tags")
        .and_then(|body| serde_json::from_str::<Value>(&body).ok())
        .map(|value| {
            value["models"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(|model| {
                    (
                        model["name"].as_str().unwrap_or("unknown").to_string(),
                        model["size"].as_f64().unwrap_or(0.0) / 1_000_000_000.0,
                    )
                })
                .collect::<Vec<_>>()
        });
    Status {
        ollama_running: models.is_some(),
        models: models.unwrap_or_default(),
        llama_running: get(8080, "/health").is_some(),
    }
}

fn selected_path() -> Option<std::path::PathBuf> {
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
    Some(base.join("LumeCode").join("local_model.txt"))
}

pub fn selected() -> Option<String> {
    let name = std::fs::read_to_string(selected_path()?).ok()?;
    let name = name.trim().to_string();
    (!name.is_empty()).then_some(name)
}

pub fn save_selected(name: &str) {
    if let Some(path) = selected_path() {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(path, name);
    }
}

pub fn report() -> String {
    let status = status();
    let mut out = String::new();
    if status.ollama_running {
        out.push_str("Ollama: running on port 11434.\n");
        if status.models.is_empty() {
            out.push_str("No Ollama models installed. Run `ollama pull <model>` to add one.\n");
        } else {
            out.push_str("Installed Ollama models:\n");
            for (name, gb) in &status.models {
                out.push_str(&format!("  {name}  ({gb:.1} GB)\n"));
            }
        }
    } else {
        out.push_str("Ollama: not running (port 11434).\n");
    }
    if status.llama_running {
        out.push_str("llama.cpp server: running on port 8080.\n");
    } else {
        out.push_str("llama.cpp server: not running (port 8080).\n");
    }
    match selected() {
        Some(name) => out.push_str(&format!("Selected local model: {name}\n")),
        None => out.push_str("No local model selected. Open /local to choose one.\n"),
    }
    out
}