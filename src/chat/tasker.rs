use serde_json::{json, Value};
#[derive(Clone)]
pub struct Task {
    pub id: String,
    pub name: String,
    pub label: String,
    pub command: String,
    pub done: bool,
    pub failed: bool,
    pub detail: String,
}
pub fn label(name: &str) -> &str {
    match name {
        "read_file" => "Reading file",
        "list_files" => "Listing files",
        "search_files" => "Searching files",
        "create_file" | "write_file_chunk" => "Writing file",
        "edit_file" => "Editing file",
        "run_command" => "Running command",
        "run_code" => "Running sandbox code",
        "web_search" => "Searching the web",
        "fetch_web" => "Reading web page",
        "update_task" => "Updating tasks",
        "get_datetime" => "Checking date and time",
        "context_compacting" | "context_compacted" => "Compacting conversation",
        "deep_search" => "Deep research",
        "get_info" => "Checking project",
        "create_folder" => "Creating folder",
        "move_file" => "Moving file",
        "delete_file" => "Deleting file",
        "compile_ZIP" => "Packaging files",
        "unzip" => "Extracting files",
        "present_file" => "Presenting file",
        "highlight" => "Highlighting useful information",
        "image_search" => "Searching images",
        "switch_mode" => "Updating mode",
        "coworker_manage" | "coworker_status" | "request_coworker_files" => "Checking Coworker",
        "tool_preparing" | "tool" | "file_preview" => "Working",
        "tool_done" | "tool_generation_failed" => "Tool activity",
        _ => "Using a connected tool",
    }
}
fn finished_label(name: &str) -> &str {
    match name {
        "read_file" => "File read",
        "list_files" => "Files listed",
        "search_files" => "Files searched",
        "create_file" | "write_file_chunk" | "edit_file" => "File saved",
        "run_command" => "Ran command",
        "run_code" => "Code finished",
        "web_search" => "Searched the web",
        "fetch_web" => "Page read",
        "context_compacting" | "context_compacted" => "Conversation compacted",
        _ => label(name),
    }
}
pub fn is_write_tool(name: &str) -> bool {
    matches!(name, "create_file" | "write_file_chunk")
}
fn target_name(v: &Value) -> Option<String> {
    v["path"]
        .as_str()
        .or_else(|| v["file"].as_str())
        .map(|p| p.rsplit(['/', '\\']).next().unwrap_or(p).to_string())
        .filter(|s| !s.is_empty())
}
fn remembered_file(label: &str) -> Option<String> {
    label
        .strip_prefix("Writing ")
        .or_else(|| label.strip_prefix("Editing "))
        .and_then(|s| s.strip_suffix('…'))
        .map(str::to_string)
}
fn running_label(name: &str, file: Option<&str>) -> String {
    match (name, file) {
        ("create_file" | "write_file_chunk", Some(f)) => format!("Writing {f}…"),
        ("edit_file", Some(f)) => format!("Editing {f}…"),
        _ => label(name).into(),
    }
}
fn done_label(name: &str, file: Option<&str>) -> String {
    match (name, file) {
        ("create_file" | "write_file_chunk", Some(f)) => format!("Created {f}"),
        ("edit_file", Some(f)) => format!("Edited {f}"),
        _ => finished_label(name).into(),
    }
}
fn status_label(name: &str, file: Option<&str>, done: bool, failed: bool) -> String {
    match (done, failed) {
        (true, true) if name == "run_command" => "Command failed".into(),
        (true, true) => format!("Stopped: {}", label(name)),
        (true, false) => done_label(name, file),
        (false, _) => running_label(name, file),
    }
}
fn readable_output(raw: &str) -> String {
    match serde_json::from_str::<Value>(raw) {
        Ok(v) if v.get("background").and_then(Value::as_bool) == Some(true) => format!(
            "Started in background ({})",
            v["job"].as_str().unwrap_or("job")
        ),
        Ok(v) if v.get("stdout").is_some() || v.get("stderr").is_some() => {
            let stdout = v["stdout"].as_str().unwrap_or("").trim_end();
            let stderr = v["stderr"].as_str().unwrap_or("").trim_end();
            [stdout, stderr]
                .into_iter()
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join("\n")
        }
        Ok(v) if v.get("content").is_some() => v["content"].as_str().unwrap_or("").to_string(),
        Ok(v) if v.get("files").is_some() => v["files"]
            .as_array()
            .map(|files| {
                files
                    .iter()
                    .filter_map(|f| f["path"].as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default(),
        Ok(v) if v.get("error").is_some() => v["error"].as_str().unwrap_or("").to_string(),
        Ok(v) if v.is_object() || v.is_array() => "Done".to_string(),
        _ => raw.to_string(),
    }
}

pub fn apply(tasks: &mut Vec<Task>, turn: usize, v: &Value) {
    if let Some(path) = std::env::var_os("LUME_DEBUG_EVENTS") {
        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = writeln!(file, "{v}");
        }
    }
    let state = v["state"].as_str().unwrap_or("");
    if !matches!(
        state,
        "tool_preparing"
            | "tool"
            | "tool_done"
            | "tool_reused"
            | "file_preview"
            | "tool_generation_failed"
            | "context_compacting"
            | "context_compacted"
    ) {
        return;
    }
    let supplied = v["name"].as_str().filter(|s| !s.is_empty());
    let name = supplied.unwrap_or(state);
    let command: String = if name == "run_command" {
        v["command"].as_str().unwrap_or("").chars().take(300).collect()
    } else {
        String::new()
    };
    let file = target_name(v);
    let prefix = format!("{turn}_");
    let id = if let Some(id) = v["activity_id"].as_str() {
        format!("{prefix}{id}")
    } else {
        tasks
            .iter()
            .rev()
            .find(|t| {
                t.id.starts_with(&prefix) && (supplied.is_none() || t.name == name) && !t.done
            })
            .map(|t| t.id.clone())
            .unwrap_or_else(|| format!("{prefix}{name}_{}", tasks.len()))
    };
    let done = matches!(
        state,
        "tool_done" | "tool_reused" | "tool_generation_failed" | "context_compacted"
    );
    let failed = v["ok"] == false || state == "tool_generation_failed";
    let raw = if state == "file_preview" {
        v["text_delta"].as_str().unwrap_or("")
    } else {
        v["output"].as_str().unwrap_or("")
    };
    let shown = readable_output(raw);
    let detail = shown.as_str();
    if let Some(t) = tasks.iter_mut().find(|t| t.id == id) {
        if let Some(name) = supplied {
            t.name = name.into();
        }
        let file = file.clone().or_else(|| remembered_file(&t.label));
        if !command.is_empty() {
            t.command = command.clone();
        }
        t.done |= done;
        t.failed |= failed;
        t.label = status_label(&t.name, file.as_deref(), t.done, t.failed);
        if state == "file_preview" {
            if v["reset"] == true {
                t.detail.clear();
            }
            t.detail.push_str(detail);
            t.detail = t.detail.chars().take(12000).collect();
        } else if !detail.is_empty() && (!is_write_tool(&t.name) || (done && failed)) {
            t.detail = detail.chars().take(4000).collect();
        }
    } else {
        tasks.push(Task {
            id,
            name: name.into(),
            label: status_label(name, file.as_deref(), done, failed),
            done,
            failed,
            detail: if is_write_tool(name) && state != "file_preview" && !failed {
                String::new()
            } else {
                detail.chars().take(4000).collect()
            },
            command,
        });
    }
}
pub fn finish(tasks: &mut [Task]) {
    for t in tasks {
        if !t.done {
            t.done = true;
            t.failed = true;
        }
    }
}
pub fn record(v: &Value) -> Value {
    let mut out = json!({});
    for key in ["state", "activity_id", "name", "ok", "reset"] {
        if let Some(value) = v.get(key) {
            out[key] = value.clone();
        }
    }
    if let Some(value) = v["output"].as_str() {
        out["output"] = json!(readable_output(value).chars().take(4000).collect::<String>());
    }
    if let Some(value) = v["text_delta"].as_str() {
        out["text_delta"] = json!(value.chars().take(4000).collect::<String>());
    }
    out
}
pub fn card_note(v: &Value) -> String {
    let mut text = String::new();
    if let Some(s) = v["highlight"]["text"].as_str() {
        text.push_str(&format!("\nHighlighted: {s}"));
    }
    if let Some(id) = v["coworker"]["id"].as_str() {
        text.push_str(&format!("\nCoworker: {id}\nUse /coworker upload ID [\"main.py\",\"data.csv\"], then /coworker start ID."));
    }
    if let Some(id) = v["research"]["id"].as_str() {
        text.push_str(&format!("\nResearch plan: {id}. Use /research show ID to review, then /research start ID or /research cancel ID."));
    }
    text
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn streamed_tool_names_update_the_existing_activity_without_ghost_rows() {
        let mut tasks = vec![];
        apply(
            &mut tasks,
            1,
            &json!({"state":"tool_preparing","activity_id":"one","name":""}),
        );
        assert_eq!(tasks[0].label, "Working");
        apply(
            &mut tasks,
            1,
            &json!({"state":"tool_preparing","activity_id":"one","name":"create_file"}),
        );
        assert_eq!(tasks[0].label, "Writing file");
        apply(
            &mut tasks,
            1,
            &json!({"state":"tool_done","activity_id":"one","ok":true}),
        );
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].label, "File saved");
    }
    #[test]
    fn file_tools_show_the_file_name_while_writing_and_after_done() {
        let mut tasks = vec![];
        apply(
            &mut tasks,
            1,
            &json!({"state":"tool_preparing","activity_id":"w","name":"create_file","path":"src/icon.svg"}),
        );
        assert_eq!(tasks[0].label, "Writing icon.svg…");
        apply(
            &mut tasks,
            1,
            &json!({"state":"tool_done","activity_id":"w","name":"create_file","ok":true}),
        );
        assert_eq!(tasks[0].label, "Created icon.svg");
    }
    #[test]
    fn tool_output_never_shows_raw_json() {
        let mut tasks = vec![];
        apply(
            &mut tasks,
            1,
            &json!({"state":"tool_done","activity_id":"f","name":"list_files","ok":true,"output":"{\"files\":[{\"directory\":false,\"path\":\"index.html\"}],\"limit\":2000}"}),
        );
        assert_eq!(tasks[0].detail, "index.html");
    }
    #[test]
    fn command_output_shows_stdout_not_raw_json() {
        let mut tasks = vec![];
        apply(
            &mut tasks,
            1,
            &json!({"state":"tool_preparing","activity_id":"c","name":"run_command","command":"echo hi"}),
        );
        apply(
            &mut tasks,
            1,
            &json!({"state":"tool_done","activity_id":"c","name":"run_command","ok":true,"output":"{\"exit_code\":0,\"output_limit\":65536,\"stderr\":\"\",\"stdout\":\"hi\\r\\n\"}"}),
        );
        assert_eq!(tasks[0].detail, "hi");
        assert_eq!(tasks[0].command, "echo hi");
        assert_eq!(tasks[0].label, "Ran command");
    }
    #[test]
    fn missing_result_ids_complete_the_right_tool_without_merging_distinct_calls() {
        let mut tasks = vec![];
        for id in ["tool_0_0", "tool_0_1"] {
            apply(
                &mut tasks,
                1,
                &json!({"state":"tool_preparing","activity_id":id,"name":"web_search"}),
            );
            apply(&mut tasks, 1, &json!({"state":"tool","name":"web_search"}));
            apply(
                &mut tasks,
                1,
                &json!({"state":"tool_done","name":"web_search","ok":true,"output":"sources"}),
            );
        }
        assert_eq!(tasks.len(), 2);
        assert!(tasks.iter().all(|t| t.done && !t.failed));
    }
}