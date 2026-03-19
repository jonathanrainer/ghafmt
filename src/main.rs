//! Binary entry point for the `ghafmt` CLI.

use std::process::ExitCode;

use clap::Parser;
use ghafmt::{Ghafmt, cli::Args};

/// Parse CLI arguments, format the given workflow file(s), and handle output
/// according to the selected mode.
fn main() -> ExitCode {
    let args = Args::parse();

    let log_level = if args.quiet {
        tracing::Level::ERROR
    } else {
        tracing::Level::WARN
    };
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_max_level(log_level)
        .init();

    Ghafmt::new().run(args.files, args.mode, args.colour, args.quiet)
}
