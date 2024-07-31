#![doc = include_str!("../README.md")]
use clap::Parser;

mod cli;

fn main() -> anyhow::Result<()> {
    let args = crate::cli::Args::parse();

    args.run()
}
