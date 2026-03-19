//! Library entry point for `ghafmt`.
//!
//! Exposes [`Ghafmt`], which orchestrates the structure-transformation and
//! presentation-transformation pipeline for GitHub Actions workflow files.
mod constants;
pub mod errors;

use std::{fs::read_to_string, process::ExitCode};

pub use errors::{Error, Result, Warning};
use patharg::InputArg;
use tracing::info;

use crate::{
    cli::{ColourMode, Mode},
    commands::Command,
    fs::{expand_paths, read_from_stdin},
    workflow_emitter::WorkflowEmitter,
    workflow_processor::WorkflowProcessor,
};

pub mod cli;
/// Implementations of the four formatting modes.
pub(crate) mod commands;
/// File-system helpers: path expansion, atomic writes, and stdin reading.
mod fs;
mod presentation_transformers;
mod structure_transformers;
mod workflow_emitter;
mod workflow_processor;

/// The formatted output and any advisory warnings produced for one file.
struct FormatterResult {
    /// Path to the source file, or `"-"` for stdin.
    input: InputArg,
    /// Formatted YAML output.
    output: String,
    /// Original content before formatting. Only `Some` for stdin, where the source
    /// cannot be re-read from disk for `--mode=check`/`--mode=list` comparisons.
    original: Option<String>,
    /// Non-fatal warnings produced during formatting.
    warnings: Vec<Warning>,
}

impl FormatterResult {
    /// Returns the original file content before formatting.
    ///
    /// For stdin, this is the captured input stored in [`FormatterResult::original`].
    /// For file paths, the file is re-read from disk on demand.
    fn original_content(&self) -> Option<String> {
        self.original
            .clone()
            .or_else(|| self.input.read_to_string().ok())
    }
}

/// Top-level formatter that processes and emits a GitHub Actions workflow file.
pub struct Ghafmt {
    /// Applies structure transformers to the parsed document.
    workflow_processor: WorkflowProcessor,
    /// Applies presentation transformers and serialises the result to YAML.
    workflow_emitter: WorkflowEmitter,
}

impl Default for Ghafmt {
    fn default() -> Self {
        Self::new()
    }
}

impl Ghafmt {
    /// Create a new `Ghafmt` instance with the default transformer pipeline.
    #[must_use]
    pub fn new() -> Self {
        Self {
            workflow_processor: WorkflowProcessor::new(),
            workflow_emitter: WorkflowEmitter::new(),
        }
    }

    #[must_use]
    pub fn run(
        mut self,
        files: Vec<InputArg>,
        mode: Mode,
        colour_mode: ColourMode,
        quiet: bool,
    ) -> ExitCode {
        if matches!(mode, Mode::Write) && files.iter().any(InputArg::is_stdin) {
            eprintln!("error: stdin (-) cannot be used with --mode=write");
            return ExitCode::FAILURE;
        }

        // Default (stdout) mode can only handle one file; all other modes accept many.
        if matches!(mode, Mode::Format) && files.len() > 1 {
            eprintln!("error: multiple files require --mode=write, --mode=check, or --mode=list");
            return ExitCode::FAILURE;
        }

        let expanded_files = expand_paths(files);

        let mut results: Vec<Result<FormatterResult>> = Vec::with_capacity(expanded_files.len());
        for file in &expanded_files {
            let (content, name) = match file {
                InputArg::Stdin => match read_from_stdin() {
                    Ok(content) => (content, "<stdin>"),
                    Err(e) => {
                        eprintln!("error: {e}");
                        return ExitCode::FAILURE;
                    }
                },
                InputArg::Path(p) => {
                    match read_to_string(p).map_err(|source| Error::ReadFile {
                        path: p.clone(),
                        source,
                    }) {
                        Ok(content) => {
                            let file_name = p
                                .file_name()
                                .and_then(|a| a.to_str())
                                .unwrap_or("<could_not_determine>");
                            (content, file_name)
                        }
                        Err(e) => {
                            eprintln!("error: {e}");
                            return ExitCode::FAILURE;
                        }
                    }
                }
            };

            let result = self
                .format_gha_workflow(&content, name)
                .map(|(output, warnings)| FormatterResult {
                    input: file.clone(),
                    output,
                    original: Some(content),
                    warnings,
                });
            results.push(result);
        }

        match mode {
            Mode::Format => commands::Format {}.run(&results, colour_mode, quiet),
            Mode::Check => commands::Check {}.run(&results, colour_mode, quiet),
            Mode::Write => commands::Write {}.run(&results, colour_mode, quiet),
            Mode::List => commands::List {}.run(&results, colour_mode, quiet),
        }
    }

    /// Format a GitHub Actions workflow from a string and return the formatted YAML
    /// along with any non-fatal warnings produced during processing.
    ///
    /// `name` is used only in diagnostic output (e.g. `"<stdin>"`).
    ///
    /// # Errors
    ///
    /// Returns an error if `content` cannot be parsed as valid YAML.
    /// Transformer failures are returned as warnings rather than errors.
    pub fn format_gha_workflow(
        &mut self,
        content: &str,
        name: &str,
    ) -> Result<(String, Vec<Warning>)> {
        info!("Beginning Document Processing...");
        let (yaml, warnings) = self.workflow_processor.process(content, name)?;
        info!("Emitting workflow...");
        let yaml_string = self.workflow_emitter.emit(&yaml)?;
        Ok((yaml_string, warnings))
    }
}
