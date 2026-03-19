use std::process::ExitCode;

use crate::{
    FormatterResult,
    cli::ColourMode,
    commands::{Command, build_handler, render_error},
};

/// Print each formatted result to stdout; exit 1 immediately on the first error.
pub(crate) struct Format {}

impl Command for Format {
    fn run(
        &self,
        results: &[crate::Result<FormatterResult>],
        colour_mode: ColourMode,
        quiet: bool,
    ) -> ExitCode {
        let handler = build_handler(colour_mode);
        for result in results {
            match result {
                Ok(success) => {
                    self.render_warnings(&handler, &success.warnings, quiet);
                    print!("{}", success.output);
                }
                Err(error) => {
                    render_error(&handler, error);
                    return ExitCode::FAILURE;
                }
            }
        }
        ExitCode::SUCCESS
    }
}
