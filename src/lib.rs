//! Library entry point for `ghafmt`.
//!
//! Exposes [`Ghafmt`], which orchestrates the structure-transformation and
//! presentation-transformation pipeline for GitHub Actions workflow files.
mod constants;
pub mod errors;

use std::{fs::read_to_string, process::ExitCode};

pub use errors::{Error, Result, Warning};
use fyaml::Document;
use patharg::InputArg;
use tracing::info;

use crate::{
    cli::{ColourMode, Mode},
    commands::{build_handler, render_error, Command},
    fs::{expand_paths, read_from_stdin},
    structure_transformers::{
        CaseEnforcer, ConcurrencySorter, ContainerSorter, DefaultsSorter, EnvSorter,
        EnvironmentSorter, FilterSorter, JobSorter, NeedsSorter, OnSorter, PermissionsSorter,
        RunsOnSorter, StepSorter, StrategySorter, StructureTransformer, TopLevelSorter, WithSorter,
        WorkflowCallSorter, WorkflowDispatchSorter, WorkflowRunSorter,
    },
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

#[derive(Eq, PartialEq, Hash)]
enum DocumentType {
    Unknown,
    Workflow,
    CompositeAction,
    DockerAction,
    JavascriptAction,
}

/// Top-level formatter that processes and emits a GitHub Actions workflow file.
pub struct Ghafmt {
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

            match Document::parse_str(&content) {
                Ok(document) => {
                    let result = self
                        .format_gha_workflow(document)
                        .map(|(output, warnings)| FormatterResult {
                            input: file.clone(),
                            output,
                            original: content,
                            warnings,
                        });
                    results.push(result);
                }
                Err(e) => results.push(Err(Error::parse_yaml(name, &content, &e))),
            }
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
    pub fn format_gha_workflow(&mut self, doc: Document) -> Result<(String, Vec<Warning>)> {
        info!("Beginning Document Processing...");
        let workflow_processor = WorkflowProcessor::new(self.get_transformers(&doc));
        let (yaml, warnings) = workflow_processor.process(doc)?;
        info!("Emitting workflow...");
        let yaml_string = self.workflow_emitter.emit(&yaml)?;
        Ok((yaml_string, warnings))
    }

    pub(crate) fn get_transformers(
        &self,
        document: &Document,
    ) -> Vec<Box<dyn StructureTransformer>> {
        let document_type = match document.at_path("/runs/using") {
            None => match document.at_path("/on") {
                None => DocumentType::Unknown,
                Some(_) => DocumentType::Workflow,
            },
            Some(_) => DocumentType::Unknown,
        };
        match document_type {
            DocumentType::Unknown => vec![],
            DocumentType::Workflow => vec![
                Box::new(TopLevelSorter::default()),
                Box::new(JobSorter::default()),
                Box::new(StepSorter::default()),
                Box::new(OnSorter::default()),
                Box::new(WorkflowDispatchSorter::default()),
                Box::new(WithSorter),
                Box::new(WorkflowCallSorter::default()),
                Box::new(WorkflowRunSorter::default()),
                Box::new(PermissionsSorter),
                Box::new(EnvSorter),
                Box::new(DefaultsSorter),
                Box::new(ConcurrencySorter::default()),
                Box::new(EnvironmentSorter::default()),
                Box::new(NeedsSorter::default()),
                Box::new(RunsOnSorter::default()),
                Box::new(FilterSorter::default()),
                Box::new(StrategySorter::default()),
                Box::new(ContainerSorter::default()),
                Box::new(CaseEnforcer::new(heck::ToSnakeCase::to_snake_case)),
            ],
            DocumentType::CompositeAction => vec![],
            DocumentType::DockerAction => vec![],
            DocumentType::JavascriptAction => vec![],
        }
    }
}
