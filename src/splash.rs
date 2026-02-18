use std::io::{self, IsTerminal, Write};
use std::thread;
use std::time::Duration;

const ESC: &str = "\x1b[";
const RESET: &str = "\x1b[0m";
const CYAN: &str = "\x1b[36m";
const BLUE: &str = "\x1b[94m";
const YELLOW: &str = "\x1b[93m";
const DIM: &str = "\x1b[2m";
const BOLD: &str = "\x1b[1m";

pub fn show_dna_splash(
    command_name: &str,
    update_info: Option<&crate::update::UpdateInfo>,
) -> io::Result<()> {
    if !io::stdout().is_terminal() {
        return Ok(());
    }

    let _cursor_guard = CursorGuard;
    let mut out = io::stdout().lock();
    write!(out, "{ESC}?25l")?;

    let total_frames = 18usize;
    for phase in 0..total_frames {
        render_frame(&mut out, phase, command_name, false, update_info)?;
        out.flush()?;
        thread::sleep(Duration::from_millis(55));
    }

    render_frame(&mut out, total_frames, command_name, true, update_info)?;
    out.flush()?;
    Ok(())
}

fn render_frame<W: Write>(
    out: &mut W,
    phase: usize,
    command_name: &str,
    final_frame: bool,
    update_info: Option<&crate::update::UpdateInfo>,
) -> io::Result<()> {
    write!(out, "{ESC}2J{ESC}H")?;
    writeln!(
        out,
        "{BOLD}{CYAN}primer-scout{RESET} {BLUE}DNA startup mode{RESET}"
    )?;
    writeln!(
        out,
        "{DIM}Fast primer off-target scanning for FASTA references{RESET}"
    )?;
    writeln!(out)?;

    for (row, line) in helix_lines(phase).into_iter().enumerate() {
        if row % 2 == 0 {
            writeln!(out, "  {CYAN}{line}{RESET}")?;
        } else {
            writeln!(out, "  {BLUE}{line}{RESET}")?;
        }
    }

    writeln!(out)?;
    if final_frame {
        writeln!(
            out,
            "{BOLD}Ready:{RESET} run scans with `{command_name} --primers <file.tsv> --reference <ref.fa> --summary`"
        )?;
        writeln!(
            out,
            "{DIM}Tip: `{command_name} --help` for full command options.{RESET}"
        )?;
        if let Some(update) = update_info {
            writeln!(
                out,
                "{YELLOW}{BOLD}Update available!{RESET} {YELLOW}v{}{RESET}",
                update.latest_version
            )?;
            writeln!(out, "{YELLOW}Run: {}{RESET}", update.install_command)?;
        }
    } else {
        let dots = ".".repeat((phase % 4) + 1);
        writeln!(out, "{DIM}Initializing helix renderer{dots}{RESET}")?;
    }
    Ok(())
}

fn helix_lines(phase: usize) -> Vec<String> {
    let width = 44usize;
    let curve = [8usize, 10, 12, 14, 12, 10, 8, 6];
    let mut lines = Vec::with_capacity(14);

    for row in 0..14usize {
        let idx = (row + phase) % curve.len();
        let left = curve[idx];
        let right = width.saturating_sub(left);
        let left_char = if idx < (curve.len() / 2) { '/' } else { '\\' };
        let right_char = if idx < (curve.len() / 2) { '\\' } else { '/' };
        let bridge_len = right.saturating_sub(left + 1);
        let bridge = if row % 2 == 0 {
            "=".repeat(bridge_len)
        } else {
            " ".repeat(bridge_len)
        };

        let mut line = String::new();
        line.push_str(&" ".repeat(left));
        line.push(left_char);
        line.push_str(&bridge);
        line.push(right_char);
        lines.push(line);
    }

    lines
}

struct CursorGuard;

impl Drop for CursorGuard {
    fn drop(&mut self) {
        let mut out = io::stdout();
        let _ = write!(out, "{RESET}{ESC}?25h");
        let _ = out.flush();
    }
}
