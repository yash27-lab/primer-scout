use anyhow::Result;
use std::env;
use std::ffi::OsStr;
use std::io::{self, IsTerminal};

fn main() -> Result<()> {
    let args: Vec<_> = env::args_os().collect();
    let wants_splash = args.len() == 1 || (args.len() == 2 && args[1] == OsStr::new("--splash"));

    if wants_splash && io::stdout().is_terminal() {
        primer_scout::splash::show_dna_splash("primer")?;
        return Ok(());
    }

    primer_scout::cli::run_from_args(args)
}
