use std::io::Write;

use anyhow::Context;

use clap::Parser;

use ignore::DirEntry;

use rayon::iter::{ParallelBridge, ParallelIterator};

use orthotypos::config::Config;
use orthotypos::lint::Linter;

mod cli;

#[allow(clippy::print_stderr)]
fn main() -> anyhow::Result<()> {
    let args = crate::cli::Args::parse();

    let report_handler = args.format().into_error_hook();
    miette::set_hook(report_handler)?;

    let cwd = std::env::current_dir().context("no current working directory")?;
    let mut config = Config::default();
    for ancestor in cwd.ancestors() {
        if let Some(derived) = Config::from_dir(ancestor)? {
            config.update(&derived);
            break;
        }
    }
    config.update(&args.to_config());

    let walker = args.to_walk(&config)?;
    let process_entry = |file: DirEntry| {
        let Ok(Some(linter)) = Linter::from_path(file.path()) else {
            return 0;
        };

        let mut stderr = std::io::stderr().lock();
        linter
            .iter()
            .map(|typo| {
                let typo: miette::Report = typo.into();
                let _ = writeln!(stderr, "{typo:?}");
            })
            .count()
    };
    let typos_found: usize = if args.sort() {
        walker.map(process_entry).sum()
    } else {
        walker.par_bridge().map(process_entry).sum()
    };

    if typos_found > 0 {
        std::process::exit(1);
    } else {
        Ok(())
    }
}
