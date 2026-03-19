use std::process::ExitCode;

use crate::{cli::ColourMode, commands::Command, fs::atomic_write, FormatterResult};

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
                    if let Err(e) = atomic_write(&success.path, &success.output) {
                        eprintln!("{}: {e}", success.path.display());
                        exit_code = ExitCode::FAILURE;
                    }
                }
                Err(error) => {
                    self.render_error(&handler, &error);
                    exit_code = ExitCode::FAILURE;
                }
            }
        }
        exit_code
    }
}
