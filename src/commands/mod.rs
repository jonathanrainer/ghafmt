/// Check mode: compare formatted output against the original and report diffs.
mod check;
/// Format mode: write formatted output to stdout.
mod format;
/// List mode: print paths of files that differ from their formatted form.
mod list;
/// Write mode: format files in place.
mod write;

use std::process::ExitCode;

pub(crate) use check::Check;
pub(crate) use format::Format;
pub(crate) use list::List;
use miette::{GraphicalReportHandler, GraphicalTheme};
pub(crate) use write::Write;

use crate::{Error, FormatterResult, Result, Warning, cli::ColourMode};

/// Build a [`GraphicalReportHandler`] according to the chosen colour mode.
///
/// `--colour always` forces colour on regardless of environment.
/// `--colour never` forces colour off.
/// `--colour auto` (the default) disables colour when `NO_COLOR` is set; otherwise
/// delegates to miette's own terminal detection.
pub(crate) fn build_handler(colour: ColourMode) -> GraphicalReportHandler {
    match colour {
        ColourMode::Always => GraphicalReportHandler::new_themed(GraphicalTheme::unicode()),
        ColourMode::Never => GraphicalReportHandler::new_themed(GraphicalTheme::unicode_nocolor()),
        ColourMode::Auto => {
            if std::env::var_os("NO_COLOR").is_some() {
                GraphicalReportHandler::new_themed(GraphicalTheme::unicode_nocolor())
            } else {
                GraphicalReportHandler::new()
            }
        }
    }
}

/// Render and print a fatal error to stderr.
pub(crate) fn render_error(handler: &GraphicalReportHandler, error: &Error) {
    let mut rendered = String::new();
    if handler.render_report(&mut rendered, error).is_ok() {
        eprintln!("{rendered}");
    } else {
        eprintln!("error: {error}");
    }
}

/// Shared interface for the four formatting modes (format, check, write, list).
pub(crate) trait Command {
    /// Execute the command over `results` and return the appropriate exit code.
    fn run(
        &self,
        results: &[Result<FormatterResult>],
        colour_mode: ColourMode,
        quiet: bool,
    ) -> ExitCode;

    /// Render and print all warnings to stderr, unless `quiet` is set.
    fn render_warnings(&self, handler: &GraphicalReportHandler, warnings: &[Warning], quiet: bool) {
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
}
