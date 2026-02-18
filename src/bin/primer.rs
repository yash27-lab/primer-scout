use anyhow::Result;
use std::env;
use std::ffi::OsStr;
use std::io::{self, IsTerminal};

fn main() -> Result<()> {
    let args: Vec<_> = env::args_os().collect();
    let wants_console = args.len() == 1 || (args.len() == 2 && args[1] == OsStr::new("--splash"));

    if wants_console && io::stdout().is_terminal() {
        let update_info = primer_scout::update::check_for_update(env!("CARGO_PKG_VERSION"));
        primer_scout::console::run("primer", update_info.as_ref())?;
        return Ok(());
    }

    primer_scout::cli::run_from_args(args)
}
