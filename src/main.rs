//! Binary entry point for the `ghafmt` CLI.
use std::{
    fs,
    fs::read_to_string,
    io::Read,
    path::{Path, PathBuf},
    process,
};

use clap::Parser;
use ghafmt::{Error, Ghafmt, Warning};
use miette::{GraphicalReportHandler, GraphicalTheme};
use similar::TextDiff;
use walkdir::WalkDir;

/// The conventional marker used to request stdin as an input source.
const STDIN: &str = "-";

/// When to use colour in diagnostic output.
#[derive(clap::ValueEnum, Clone, Copy, Debug)]
enum ColorMode {
    /// Always use colour, even when not writing to a terminal.
    Always,
    /// Use colour when writing to a terminal and `NO_COLOR` is not set.
    Auto,
    /// Never use colour.
    Never,
}

/// Format GitHub Actions workflow files.
#[derive(Parser, Debug)]
#[allow(clippy::struct_excessive_bools)]
#[command(
    author,
    version,
    about,
    after_help = "\
Examples:
  # Format a single file and print to stdout
  ghafmt .github/workflows/ci.yml

  # Format stdin and print to stdout
  cat ci.yml | ghafmt -

  # Check whether files are formatted (useful in CI)
  ghafmt --check .github/workflows/ci.yml

  # Check whether stdin is formatted
  cat ci.yml | ghafmt --check -

  # Write formatted output back to files in place
  ghafmt --write .github/workflows/ci.yml .github/workflows/release.yml

  # Format all workflow files under a directory
  ghafmt --write .github/workflows/

  # List files that need formatting
  ghafmt --list .github/workflows/"
)]
struct Args {
    /// Workflow files or directories to format. Use `-` to read from stdin.
    files: Vec<PathBuf>,
    /// Check whether files are formatted; exit 1 if any differ printing a diff to stderr.
    #[arg(long, group = "mode")]
    check: bool,
    /// Write formatted output back to each file in place.
    #[arg(long, short = 'w', group = "mode")]
    write: bool,
    /// List files that differ from their formatted form; exit 1 if any are found.
    #[arg(long, short = 'l', group = "mode")]
    list: bool,
    /// Suppress warnings; errors are still reported.
    #[arg(long, short = 'q')]
    quiet: bool,
    /// Control colour in diagnostic output.
    #[arg(long, value_enum, default_value_t = ColorMode::Auto)]
    color: ColorMode,
}

/// The formatted output and any advisory warnings produced for one file.
struct Success {
    /// Path to the source file, or `"-"` for stdin.
    path: PathBuf,
    /// Formatted YAML output.
    output: String,
    /// Original content before formatting. Only `Some` for stdin, where the source
    /// cannot be re-read from disk for `--check`/`--list` comparisons.
    original: Option<String>,
    /// Non-fatal warnings produced during formatting.
    warnings: Vec<Warning>,
}

/// Returns `true` if `path` is the stdin marker (`-`).
fn is_stdin(path: &Path) -> bool {
    path.as_os_str() == STDIN
}

/// Build a [`GraphicalReportHandler`] according to the chosen colour mode.
///
/// `--color always` forces colour on regardless of environment.
/// `--color never` forces colour off.
/// `--color auto` (the default) disables colour when `NO_COLOR` is set; otherwise
/// delegates to miette's own terminal detection.
fn build_handler(color: ColorMode) -> GraphicalReportHandler {
    match color {
        ColorMode::Always => GraphicalReportHandler::new_themed(GraphicalTheme::unicode()),
        ColorMode::Never => GraphicalReportHandler::new_themed(GraphicalTheme::unicode_nocolor()),
        ColorMode::Auto => {
            if std::env::var_os("NO_COLOR").is_some() {
                GraphicalReportHandler::new_themed(GraphicalTheme::unicode_nocolor())
            } else {
                GraphicalReportHandler::new()
            }
        }
    }
}

/// Expand any directories in `paths` to their contained `*.yml`/`*.yaml` files,
/// leaving non-directory paths unchanged. Results within each directory are
/// sorted for deterministic ordering.
fn expand_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut expanded = Vec::new();
    for path in paths {
        if path.is_dir() {
            let mut dir_files: Vec<PathBuf> = WalkDir::new(&path)
                .follow_links(true)
                .into_iter()
                .filter_map(Result::ok)
                .filter(|e| e.file_type().is_file())
                .map(walkdir::DirEntry::into_path)
                .filter(|p| {
                    matches!(
                        p.extension().and_then(|ext| ext.to_str()),
                        Some("yml" | "yaml")
                    )
                })
                .collect();
            dir_files.sort();
            expanded.extend(dir_files);
        } else {
            expanded.push(path);
        }
    }
    expanded
}

/// Render and print all warnings to stderr, unless `quiet` is set.
fn render_warnings(handler: &GraphicalReportHandler, warnings: &[Warning], quiet: bool) {
    if quiet {
        return;
    }
    for warning in warnings {
        let mut rendered = String::new();
        if handler.render_report(&mut rendered, warning).is_ok() {
            eprintln!("{rendered}");
        }
    }
}

/// Render and print a fatal error to stderr.
fn render_error(handler: &GraphicalReportHandler, error: &Error) {
    let mut rendered = String::new();
    if handler.render_report(&mut rendered, error).is_ok() {
        eprintln!("{rendered}");
    } else {
        eprintln!("error: {error}");
    }
}

/// Return the original content to compare against for `--check`/`--list`.
///
/// For stdin sources the content was captured at format time and is stored on
/// `success`; for regular files it is re-read from disk here.
fn original_content(success: &Success) -> Option<String> {
    success
        .original
        .clone()
        .or_else(|| read_to_string(&success.path).ok())
}

/// Compare each result to its original; return 1 if any file differs or errored.
fn run_check(
    results: &[Result<Success, Error>],
    handler: &GraphicalReportHandler,
    quiet: bool,
) -> i32 {
    let mut exit_code = 0;
    for result in results {
        match result {
            Ok(success) => {
                render_warnings(handler, &success.warnings, quiet);
                if let Some(orig) = original_content(success)
                    && orig != success.output
                {
                    eprintln!("--- {}", success.path.display());
                    eprintln!("+++ {}\t(formatted)", success.path.display());
                    eprintln!(
                        "{}",
                        TextDiff::from_lines(orig.as_str(), success.output.as_str()).unified_diff()
                    );
                    exit_code = 1;
                }
            }
            Err(error) => {
                render_error(handler, error);
                exit_code = 1;
            }
        }
    }
    exit_code
}

/// Write each formatted result back to its source file; return 1 if any failed.
fn run_write(
    results: Vec<Result<Success, Error>>,
    handler: &GraphicalReportHandler,
    quiet: bool,
) -> i32 {
    let mut exit_code = 0;
    for result in results {
        match result {
            Ok(success) => {
                render_warnings(handler, &success.warnings, quiet);
                if let Err(e) = fs::write(&success.path, &success.output) {
                    eprintln!("{}: {e}", success.path.display());
                    exit_code = 1;
                }
            }
            Err(error) => {
                render_error(handler, &error);
                exit_code = 1;
            }
        }
    }
    exit_code
}

/// Print the path of each file that differs from its formatted form; return 1 if any do.
fn run_list(results: &[Result<Success, Error>], handler: &GraphicalReportHandler) -> i32 {
    let mut exit_code = 0;
    for result in results {
        match result {
            Ok(success) => {
                if let Some(orig) = original_content(success)
                    && orig != success.output
                {
                    println!("{}", success.path.display());
                    exit_code = 1;
                }
            }
            Err(error) => {
                render_error(handler, error);
                exit_code = 1;
            }
        }
    }
    exit_code
}

/// Print each formatted result to stdout; exit 1 immediately on the first error.
fn run_default(
    results: Vec<Result<Success, Error>>,
    handler: &GraphicalReportHandler,
    quiet: bool,
) -> i32 {
    for result in results {
        match result {
            Ok(success) => {
                render_warnings(handler, &success.warnings, quiet);
                print!("{}", success.output);
            }
            Err(error) => {
                render_error(handler, &error);
                return 1;
            }
        }
    }
    0
}

/// Parse CLI arguments, format the given workflow file(s), and handle output
/// according to the selected mode.
fn main() {
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

    let files = expand_paths(args.files);

    if args.write && files.iter().any(|f| is_stdin(f)) {
        eprintln!("error: stdin (-) cannot be used with --write");
        process::exit(1);
    }

    // Default (stdout) mode can only handle one file; all other modes accept many.
    if (!args.check && !args.write && !args.list) && files.len() > 1 {
        eprintln!("error: multiple files require --write, --check, or --list");
        process::exit(1);
    }

    let mut formatter = Ghafmt::new();
    let handler = build_handler(args.color);

    let mut results: Vec<Result<Success, Error>> = Vec::with_capacity(files.len());
    for file in &files {
        let result = if is_stdin(file) {
            let mut content = String::new();
            if let Err(source) = std::io::stdin().read_to_string(&mut content) {
                Err(Error::ReadFile {
                    path: file.clone(),
                    source,
                })
            } else {
                let original = content.clone();
                formatter
                    .format_str(content, "<stdin>")
                    .map(|(output, warnings)| Success {
                        path: file.clone(),
                        output,
                        original: Some(original),
                        warnings,
                    })
            }
        } else {
            formatter
                .format_gha_workflow(file)
                .map(|(output, warnings)| Success {
                    path: file.clone(),
                    output,
                    original: None,
                    warnings,
                })
        };
        results.push(result);
    }

    let exit_code = if args.check {
        run_check(&results, &handler, args.quiet)
    } else if args.write {
        run_write(results, &handler, args.quiet)
    } else if args.list {
        run_list(&results, &handler)
    } else {
        run_default(results, &handler, args.quiet)
    };

    process::exit(exit_code);
}
