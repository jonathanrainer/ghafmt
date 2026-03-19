use std::process::ExitCode;

use crate::{cli::ColourMode, commands::Command, FormatterResult};

/// Print the path of each file that differs from its formatted form; return 1 if any do.
pub(crate) struct List {}

impl Command for List {
    fn run(
        &self,
        results: &[crate::Result<FormatterResult>],
        colour_mode: ColourMode,
        _quiet: bool,
    ) -> ExitCode {
        let handler = self.build_handler(colour_mode);

        let (successes, failures): (Vec<_>, Vec<_>) = results.iter().partition(|a| a.is_ok());

        if !failures.is_empty() {
            for failure in failures {
                if let Err(e) = failure {
                    self.render_error(&handler, e);
                }
            }
            return ExitCode::FAILURE;
        }

        let mut exit_code = ExitCode::SUCCESS;

        for formatter_result in successes.into_iter().flatten() {
            if let Some(orig) = formatter_result.original_content()
                && orig != formatter_result.output
            {
                println!("{:}", formatter_result.input);
                exit_code = ExitCode::FAILURE;
            }
        }

        exit_code
    }
}
