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
    commands::{Command, build_handler, render_error},
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
pub(crate) struct FormatterResult {
    /// Path to the source file, or `"-"` for stdin.
    pub(crate) input: InputArg,
    /// Formatted YAML output.
    pub(crate) output: String,
    /// Original content before formatting, captured at read time.
    pub(crate) original: String,
    /// Non-fatal warnings produced during formatting.
    pub(crate) warnings: Vec<Warning>,
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
            workflow_processor: WorkflowProcessor::default(),
            workflow_emitter: WorkflowEmitter::new(),
        }
    }

    #[must_use]
    pub fn run(
        &mut self,
        files: Vec<InputArg>,
        mode: Mode,
        colour_mode: ColourMode,
        quiet: bool,
    ) -> ExitCode {
        let handler = build_handler(colour_mode);

        if matches!(mode, Mode::Write) && files.iter().any(InputArg::is_stdin) {
            render_error(&handler, &Error::StdinCannotBeUsedWithWrite);
            return ExitCode::FAILURE;
        }

        if matches!(mode, Mode::List) && files.iter().any(InputArg::is_stdin) {
            render_error(&handler, &Error::StdinCannotBeUsedWithList);
            return ExitCode::FAILURE;
        }

        // Default (stdout) mode can only handle one file; all other modes accept many.
        if matches!(mode, Mode::Format) && files.len() > 1 {
            render_error(&handler, &Error::MultipleFilesNotValidInDefaultMode);
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
                    original: content,
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
