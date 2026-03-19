use std::process::ExitCode;

use patharg::InputArg;

use crate::{FormatterResult, cli::ColourMode, commands::Command, fs::atomic_write};

/// Write each formatted result back to its source file; return 1 if any failed.
pub(crate) struct Write {}

impl Command for Write {
    fn run(
        &self,
        results: &[crate::Result<FormatterResult>],
        colour_mode: ColourMode,
        quiet: bool,
    ) -> ExitCode {
        let mut exit_code = ExitCode::SUCCESS;
        let handler = self.build_handler(colour_mode);
        for result in results {
            match result {
                Ok(success) => {
                    self.render_warnings(&handler, &success.warnings, quiet);
                    if let InputArg::Path(p) = &success.input
                        && let Err(e) = atomic_write(p, &success.output)
                    {
                        eprintln!("{}: {e}", success.input);
                        exit_code = ExitCode::FAILURE;
                    }
                }
                Err(error) => {
                    self.render_error(&handler, error);
                    exit_code = ExitCode::FAILURE;
                }
            }
        }
        exit_code
    }
}
