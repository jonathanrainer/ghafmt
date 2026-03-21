use clap::{Parser, ValueEnum};
use patharg::InputArg;
use strum::Display;

/// Format GitHub Actions workflow files.
#[derive(Parser, Debug)]
#[command(
    author,
    version = env!("GHAFMT_VERSION_STRING"),
    about,
    after_help = "\
Examples:
  # Format a single file and print to stdout
  ghafmt .github/workflows/ci.yml

  # Format stdin and print to stdout
  cat ci.yml | ghafmt -

  # Check whether files are formatted (useful in CI)
  ghafmt --mode=check .github/workflows/ci.yml

  # Check whether stdin is formatted
  cat ci.yml | ghafmt --mode=check -

  # Write formatted output back to files in place
  ghafmt --mode=write .github/workflows/ci.yml .github/workflows/release.yml

  # Format all workflow files under a directory
  ghafmt --mode=write .github/workflows/

  # List files that need formatting
  ghafmt --mode=list .github/workflows/"
)]
pub struct Args {
    /// Workflow files or directories to format. Use `-` to read from stdin.
    pub files: Vec<InputArg>,
    /// Mode to Operate in, i.e. what function should the formatter perform
    #[arg(default_value_t, long, short = 'm')]
    pub mode: Mode,
    /// Suppress warnings; errors are still reported.
    #[arg(long, short = 'q')]
    pub quiet: bool,
    /// Control colour in diagnostic output.
    #[arg(long, value_enum, default_value_t = ColourMode::Auto)]
    pub colour: ColourMode,
}

#[derive(ValueEnum, Clone, Default, Debug, Copy, Display)]
#[strum(serialize_all = "kebab_case")]
pub enum Mode {
    #[default]
    Format,
    Check,
    Write,
    List,
}

/// When to use colour in diagnostic output.
#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum ColourMode {
    /// Always use colour, even when not writing to a terminal.
    Always,
    /// Use colour when writing to a terminal and `NO_COLOR` is not set.
    Auto,
    /// Never use colour.
    Never,
}
