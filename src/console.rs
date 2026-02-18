use crate::update::UpdateInfo;
use crossterm::cursor::MoveTo;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor};
use crossterm::terminal::{
    self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
    enable_raw_mode,
};
use crossterm::{execute, queue};
use serde::{Deserialize, Serialize};
use std::cmp::min;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

const MAX_HISTORY_ITEMS: usize = 300;
const MAX_RENDERED_ITEMS: usize = 120;
const UPGRADE_COMMAND: &str =
    "cargo install --git https://github.com/yash27-lab/primer-scout --branch main --force";
const CONSOLE_COMMANDS: &[(&str, &str)] = &[
    ("/help", "show all commands"),
    ("/basics", "beginner quickstart"),
    ("/examples", "more examples"),
    ("/scan", "run scan engine"),
    ("/upgrade", "print upgrade command"),
    ("/version", "show installed version"),
    ("/history", "show session history path"),
    ("/clear", "clear current console"),
    ("/exit", "save and quit"),
];

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Role {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Entry {
    role: Role,
    text: String,
}

pub fn run(command_name: &str, update_info: Option<&UpdateInfo>) -> io::Result<()> {
    let history_path = resolve_history_path();
    let mut entries = load_entries(&history_path).unwrap_or_default();

    if entries.is_empty() {
        push_beginner_banner(&mut entries);
    } else {
        entries.push(Entry {
            role: Role::System,
            text: "Previous session restored.".to_string(),
        });
    }

    let _guard = TerminalGuard::enter()?;
    let mut stdout = io::stdout();
    let mut input = String::new();
    let update_line = update_info.map(|u| {
        format!(
            "Update available: v{} | Run: {}",
            u.latest_version, u.install_command
        )
    });

    loop {
        draw(
            &mut stdout,
            command_name,
            &entries,
            &input,
            update_line.as_deref(),
        )?;

        if !event::poll(Duration::from_millis(150))? {
            continue;
        }

        let Event::Key(key) = event::read()? else {
            continue;
        };

        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            entries.push(Entry {
                role: Role::System,
                text: "Session saved. Bye.".to_string(),
            });
            save_entries(&history_path, &entries)?;
            break;
        }

        match key.code {
            KeyCode::Char(ch) => {
                input.push(ch);
            }
            KeyCode::Backspace => {
                input.pop();
            }
            KeyCode::Enter => {
                let submitted = input.trim().to_string();
                input.clear();

                if submitted.is_empty() {
                    continue;
                }

                if submitted == "x" || submitted.eq_ignore_ascii_case("/exit") {
                    entries.push(Entry {
                        role: Role::System,
                        text: "Session saved. Bye.".to_string(),
                    });
                    save_entries(&history_path, &entries)?;
                    break;
                }

                handle_message(submitted, &mut entries);
                trim_entries(&mut entries, MAX_HISTORY_ITEMS);
                save_entries(&history_path, &entries)?;
            }
            _ => {}
        }
    }

    Ok(())
}

fn handle_message(message: String, entries: &mut Vec<Entry>) {
    entries.push(Entry {
        role: Role::User,
        text: message.clone(),
    });

    if message == "/help" {
        push_help(entries);
        return;
    }

    if message == "/basics" || message == "/start" {
        push_basics(entries);
        return;
    }

    if message == "/examples" {
        push_examples(entries);
        return;
    }

    if message == "/upgrade" {
        entries.push(Entry {
            role: Role::Assistant,
            text: format!("Run this command in shell:\n{UPGRADE_COMMAND}"),
        });
        return;
    }

    if message == "/version" {
        entries.push(Entry {
            role: Role::Assistant,
            text: format!("primer-scout version: {}", env!("CARGO_PKG_VERSION")),
        });
        return;
    }

    if message == "/history" {
        entries.push(Entry {
            role: Role::Assistant,
            text: format!("History file: {}", resolve_history_path().display()),
        });
        return;
    }

    if message == "/clear" {
        entries.clear();
        entries.push(Entry {
            role: Role::Assistant,
            text: "Console cleared. Session continues.".to_string(),
        });
        return;
    }

    if message == "primer" || message == "primer --splash" {
        entries.push(Entry {
            role: Role::Assistant,
            text: "You are already inside primer console. Use /scan <args> or /help.".to_string(),
        });
        return;
    }

    if let Some(scan_args) = message.strip_prefix("/scan") {
        let arg_str = scan_args.trim();
        if arg_str.is_empty() {
            entries.push(Entry {
                role: Role::Assistant,
                text: "Usage: /scan --primers <file.tsv> --reference <ref.fa> [flags]".to_string(),
            });
            return;
        }

        run_scan_with_args(parse_cli_args(arg_str), entries);
        return;
    }

    if let Some(args) = parse_direct_scan_args(&message) {
        run_scan_with_args(args, entries);
        return;
    }

    entries.push(Entry {
        role: Role::Assistant,
        text: "Unknown command. Use /help to see available commands.".to_string(),
    });
}

fn push_beginner_banner(entries: &mut Vec<Entry>) {
    entries.push(Entry {
        role: Role::Assistant,
        text: "Welcome to primer console.".to_string(),
    });
    entries.push(Entry {
        role: Role::Assistant,
        text: "Type /basics for beginner quickstart or /help for full command list.".to_string(),
    });
    push_basics(entries);
}

fn push_help(entries: &mut Vec<Entry>) {
    entries.push(Entry {
        role: Role::Assistant,
        text: "Commands:\n/help\n/basics\n/examples\n/scan <args>\n/upgrade\n/version\n/history\n/clear\nx or /exit"
            .to_string(),
    });
    entries.push(Entry {
        role: Role::Assistant,
        text: "You can use /scan ... OR direct command style: `primer-scout --help` or `--primers ... --reference ...`"
            .to_string(),
    });
}

fn push_basics(entries: &mut Vec<Entry>) {
    entries.push(Entry {
        role: Role::Assistant,
        text: "Beginner quickstart:\n1) /scan --primers data/demo_primers.tsv --reference data/demo.fa --count-only\n2) /scan --primers data/demo_primers.tsv --reference data/demo.fa --summary\n3) /scan --primers data/demo_primers.tsv --reference data/demo.fa --max-mismatches 1"
            .to_string(),
    });
    entries.push(Entry {
        role: Role::Assistant,
        text: "Need more? run /examples. Exit with Ctrl+C or x.".to_string(),
    });
}

fn push_examples(entries: &mut Vec<Entry>) {
    entries.push(Entry {
        role: Role::Assistant,
        text: "Examples:\n/scan --primers data/demo_primers.tsv --reference data/demo.fa --json\n/scan --primers data/demo_primers.tsv --reference data/demo.fa --no-revcomp\n/scan --primers data/demo_primers.tsv --reference data/demo.fa --max-mismatches 2 --summary"
            .to_string(),
    });
}

fn parse_cli_args(arg_str: &str) -> Vec<String> {
    arg_str
        .split_whitespace()
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>()
}

fn parse_direct_scan_args(message: &str) -> Option<Vec<String>> {
    let trimmed = message.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(rest) = trimmed.strip_prefix("primer-scout") {
        return Some(parse_cli_args(rest.trim()));
    }

    if let Some(rest) = trimmed.strip_prefix("primer ") {
        let rest = rest.trim();
        if rest.starts_with('-') {
            return Some(parse_cli_args(rest));
        }
    }

    if trimmed.starts_with('-') {
        return Some(parse_cli_args(trimmed));
    }

    if trimmed.contains("--primers") || trimmed.contains("--reference") {
        return Some(parse_cli_args(trimmed));
    }

    None
}

fn run_scan_with_args(args: Vec<String>, entries: &mut Vec<Entry>) {
    match Command::new("primer-scout").args(&args).output() {
        Ok(output) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let text = summarize_output(stdout.trim(), "Scan completed.");
                entries.push(Entry {
                    role: Role::Assistant,
                    text,
                });
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let text = summarize_output(stderr.trim(), "Scan failed.");
                entries.push(Entry {
                    role: Role::Assistant,
                    text: format!("Scan error: {text}"),
                });
            }
        }
        Err(_) => {
            entries.push(Entry {
                role: Role::Assistant,
                text: "Could not run `primer-scout` from console. Install binary in PATH first."
                    .to_string(),
            });
        }
    }
}

fn summarize_output(raw: &str, fallback: &str) -> String {
    if raw.is_empty() {
        return fallback.to_string();
    }

    let mut out = String::new();
    for (idx, line) in raw.lines().enumerate() {
        if idx >= 8 {
            out.push_str("\n... (truncated)");
            break;
        }
        if idx > 0 {
            out.push('\n');
        }
        out.push_str(line);
    }
    out
}

fn draw(
    out: &mut io::Stdout,
    command_name: &str,
    entries: &[Entry],
    input: &str,
    update_line: Option<&str>,
) -> io::Result<()> {
    let (cols, rows) = terminal::size()?;
    let cols_usize = cols as usize;
    let rows_usize = rows as usize;

    queue!(out, MoveTo(0, 0), Clear(ClearType::All))?;
    queue!(
        out,
        SetAttribute(Attribute::Bold),
        SetForegroundColor(Color::Cyan),
        Print("primer-scout"),
        ResetColor,
        SetAttribute(Attribute::Reset),
        Print("  console"),
        MoveTo(0, 1),
        SetForegroundColor(Color::DarkGrey),
        Print(format!(
            "Type /help. Exit with Ctrl+C or x. History saved in {}",
            resolve_history_path().display()
        )),
        ResetColor
    )?;

    if let Some(line) = update_line {
        queue!(
            out,
            MoveTo(0, 2),
            SetForegroundColor(Color::Yellow),
            Print(line),
            ResetColor
        )?;
    }

    let separator_row = if update_line.is_some() { 3 } else { 2 };
    queue!(
        out,
        MoveTo(0, separator_row),
        SetForegroundColor(Color::DarkGrey),
        Print("─".repeat(cols_usize)),
        ResetColor
    )?;

    let input_row = rows.saturating_sub(1);
    queue!(
        out,
        MoveTo(0, input_row.saturating_sub(1)),
        SetForegroundColor(Color::DarkGrey),
        Print("─".repeat(cols_usize)),
        ResetColor
    )?;

    let message_top = separator_row.saturating_add(1);
    let suggestion_lines = build_suggestion_lines(input, cols_usize.saturating_sub(1));
    let suggestion_rows = suggestion_lines.len() as u16;
    let message_bottom = input_row.saturating_sub(2 + suggestion_rows);
    let available_rows = message_bottom.saturating_sub(message_top).saturating_add(1) as usize;

    let wrapped = flatten_entries(entries, cols_usize.saturating_sub(2));
    let start = wrapped
        .len()
        .saturating_sub(min(available_rows, MAX_RENDERED_ITEMS));
    for (idx, line) in wrapped[start..].iter().enumerate() {
        let y = message_top + idx as u16;
        if y > message_bottom {
            break;
        }
        queue!(out, MoveTo(0, y), Print(line))?;
    }

    if !suggestion_lines.is_empty() {
        let start_row = input_row.saturating_sub(1 + suggestion_rows);
        for (idx, line) in suggestion_lines.iter().enumerate() {
            queue!(
                out,
                MoveTo(0, start_row + idx as u16),
                SetForegroundColor(Color::DarkGrey),
                Print(line),
                ResetColor
            )?;
        }
    }

    let prompt = format!("{command_name}> {input}");
    let clipped = clip_to_width(&prompt, cols_usize.saturating_sub(1));
    queue!(
        out,
        MoveTo(0, input_row),
        SetForegroundColor(Color::Cyan),
        Print(clipped),
        ResetColor
    )?;

    out.flush()?;
    let _ = rows_usize;
    Ok(())
}

fn build_suggestion_lines(input: &str, width: usize) -> Vec<String> {
    if !input.starts_with('/') {
        return Vec::new();
    }

    let typed = input.to_ascii_lowercase();
    let mut matches = CONSOLE_COMMANDS
        .iter()
        .filter(|(cmd, _)| {
            if typed == "/" {
                true
            } else {
                cmd.starts_with(&typed) || cmd.contains(&typed)
            }
        })
        .map(|(cmd, desc)| format!("{cmd:<10} {desc}"))
        .collect::<Vec<_>>();

    if matches.is_empty() {
        return vec![clip_to_width("suggestions: no matching command", width)];
    }

    matches.truncate(3);
    matches
        .into_iter()
        .enumerate()
        .map(|(idx, row)| {
            if idx == 0 {
                clip_to_width(&format!("suggestions: {row}"), width)
            } else {
                clip_to_width(&format!("             {row}"), width)
            }
        })
        .collect()
}

fn flatten_entries(entries: &[Entry], width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for entry in entries {
        let prefix = match entry.role {
            Role::User => "you",
            Role::Assistant => "primer",
            Role::System => "system",
        };

        let wrapped = wrap_text(&entry.text, width.saturating_sub(10).max(10));
        for (idx, segment) in wrapped.into_iter().enumerate() {
            if idx == 0 {
                lines.push(format!("{prefix:>6}: {segment}"));
            } else {
                lines.push(format!("{:>6}  {segment}", ""));
            }
        }
    }
    lines
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![String::new()];
    }

    let mut out = Vec::new();
    for raw_line in text.lines() {
        if raw_line.len() <= width {
            out.push(raw_line.to_string());
            continue;
        }

        let mut line = String::new();
        for word in raw_line.split_whitespace() {
            if line.is_empty() {
                line.push_str(word);
                continue;
            }
            if line.len() + 1 + word.len() <= width {
                line.push(' ');
                line.push_str(word);
            } else {
                out.push(line);
                line = word.to_string();
            }
        }
        if !line.is_empty() {
            out.push(line);
        }
    }

    if out.is_empty() {
        out.push(String::new());
    }
    out
}

fn clip_to_width(text: &str, width: usize) -> String {
    if text.len() <= width {
        text.to_string()
    } else {
        text.chars().take(width).collect()
    }
}

fn resolve_history_path() -> PathBuf {
    if let Ok(path) = env::var("PRIMER_SCOUT_SESSION_FILE") {
        return PathBuf::from(path);
    }
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".primer-scout")
        .join("console_history.ndjson")
}

fn load_entries(path: &PathBuf) -> io::Result<Vec<Entry>> {
    let content = fs::read_to_string(path)?;
    let mut entries = Vec::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(entry) = serde_json::from_str::<Entry>(line) {
            entries.push(entry);
        }
    }
    trim_entries(&mut entries, MAX_HISTORY_ITEMS);
    Ok(entries)
}

fn save_entries(path: &PathBuf, entries: &[Entry]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::File::create(path)?;
    for entry in entries {
        let line = serde_json::to_string(entry)
            .map_err(|e| io::Error::other(format!("serialize history failed: {e}")))?;
        writeln!(file, "{line}")?;
    }
    Ok(())
}

fn trim_entries(entries: &mut Vec<Entry>, max_items: usize) {
    if entries.len() > max_items {
        let drop_count = entries.len() - max_items;
        entries.drain(0..drop_count);
    }
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}
