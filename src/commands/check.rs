use std::process::ExitCode;

use similar::TextDiff;

use crate::{
    FormatterResult,
    cli::ColourMode,
    commands::{Command, build_handler, render_error},
    errors::Result,
};

/// Compare each result to its original; return 1 if any file differs or errored.
pub(crate) struct Check {}

impl Command for Check {
    fn run(
        &self,
        results: &[Result<FormatterResult>],
        colour_mode: ColourMode,
        quiet: bool,
    ) -> ExitCode {
        let handler = build_handler(colour_mode);
        let (successes, failures): (Vec<_>, Vec<_>) = results.iter().partition(|a| a.is_ok());

        if !failures.is_empty() {
            for failure in failures {
                if let Err(e) = failure {
                    render_error(&handler, e);
                }
            }
            return ExitCode::FAILURE;
        }

        let mut exit_code = ExitCode::SUCCESS;

        for formatter_result in successes.into_iter().flatten() {
            self.render_warnings(&handler, &formatter_result.warnings, quiet);
            if formatter_result.original != formatter_result.output {
                eprintln!("--- {:#}", formatter_result.input);
                eprintln!("+++ {:#}\t(formatted)", formatter_result.input);
                eprintln!(
                    "{}",
                    TextDiff::from_lines(
                        formatter_result.original.as_str(),
                        formatter_result.output.as_str()
                    )
                    .unified_diff()
                );
                exit_code = ExitCode::FAILURE;
            }
        }

        exit_code
    }
}
