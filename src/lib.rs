//! Library entry point for `ghafmt`.
//!
//! Exposes [`Ghafmt`], which orchestrates the structure-transformation and
//! presentation-transformation pipeline for GitHub Actions files.
mod constants;
pub mod errors;

use std::{fs::read_to_string, process::ExitCode};

pub use errors::{Error, Result, Warning};
use fyaml::Document;
use patharg::InputArg;
use strum::Display;
use tracing::{info, warn};

use crate::{
    cli::{ColourMode, Mode},
    commands::{build_handler, render_error, Command},
    constants::{
        COMPOSITE_KEY_ORDER, DOCKER_KEY_ORDER, JAVASCRIPT_KEY_ORDER,
        TOP_LEVEL_METADATA_KEY_ORDERING,
    },
    fs::{expand_paths, read_from_stdin},
    presentation_transformers::{
        JobsBlankLines, PresentationTransformer, StepsBlankLines, TopLevelBlankLines,
        TopLevelCommentSpacer, VariableSpacer,
    },
    structure_transformers::{
        BrandingSorter, CaseEnforcer, ConcurrencySorter, ContainerSorter, DefaultsSorter,
        EnvSorter, EnvironmentSorter, FilterSorter, InputsSorter, JobSorter, NeedsSorter, OnSorter,
        OutputsSorter, PermissionsSorter, RunsOnSorter, RunsSorter, StepSorter, StrategySorter,
        StructureTransformer, TopLevelSorter, WithSorter, WorkflowCallSorter,
        WorkflowDispatchSorter, WorkflowRunSorter,
    },
    presentation_pipeline::PresentationPipeline,
    structure_pipeline::StructurePipeline,
};

pub mod cli;
/// Implementations of the four formatting modes.
pub(crate) mod commands;
/// File-system helpers: path expansion, atomic writes, and stdin reading.
mod fs;
mod presentation_transformers;
mod structure_transformers;
mod presentation_pipeline;
mod structure_pipeline;

/// Structure and presentation transformer pipelines for a single document.
pub(crate) type TransformerPipeline = (
    Vec<Box<dyn StructureTransformer>>,
    Vec<Box<dyn PresentationTransformer>>,
);

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

#[derive(Eq, PartialEq, Hash, Display)]
/// Discriminates between file types that require different transformer pipelines.
enum DocumentType {
    /// No recognised top-level key found; pass through with presentation transforms only.
    Unknown,
    /// Standard GitHub Actions workflow file (has an `on:` key).
    Workflow,
    /// Composite action (`runs.using: composite`).
    CompositeAction,
    /// Docker container action (`runs.using: docker`).
    DockerAction,
    /// JavaScript action (`runs.using: node*`).
    JavascriptAction,
}

/// Top-level formatter that processes and emits GitHub Actions files.
#[derive(Default)]
pub struct Ghafmt {}

impl Ghafmt {
    /// Create a new `Ghafmt` instance with the default transformer pipeline.
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
                    let result =
                        self.format_gha_document(document, file)
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

    /// Format a GitHub Actions document and return the formatted YAML
    /// along with any non-fatal warnings produced during processing.
    ///
    /// `name` is used only in diagnostic output (e.g. `"<stdin>"`).
    ///
    /// # Errors
    ///
    /// Returns an error if `content` cannot be parsed as valid YAML.
    /// Transformer failures are returned as warnings rather than errors.
    pub fn format_gha_document(
        &mut self,
        doc: Document,
        path: &InputArg,
    ) -> Result<(String, Vec<Warning>)> {
        info!("Beginning Document Processing...");
        let (structure_transformers, presentation_transformers) =
            Ghafmt::get_transformers(&doc, path);
        let pipeline = StructurePipeline::new(structure_transformers);
        let (yaml, warnings) = pipeline.process(doc)?;
        info!("Emitting document...");
        let mut emitter = PresentationPipeline::new(presentation_transformers);
        let yaml_string = emitter.emit(&yaml)?;
        Ok((yaml_string, warnings))
    }

    /// Detects the document type and returns the structure and presentation transformer pipelines.
    #[allow(clippy::too_many_lines)]
    pub(crate) fn get_transformers(document: &Document, path: &InputArg) -> TransformerPipeline {
        let document_type = match document.at_path("/runs/using") {
            None => match document.at_path("/on") {
                None => {
                    warn!(
                        "Could not find 'runs/using' or '/on' in the document, defaulting to Unknown"
                    );
                    DocumentType::Unknown
                }
                Some(_) => DocumentType::Workflow,
            },
            Some(using) => match using.scalar_str() {
                Ok(key) => match key {
                    "node20" | "node24" => DocumentType::JavascriptAction,
                    "composite" => DocumentType::CompositeAction,
                    "docker" => DocumentType::DockerAction,
                    s => {
                        warn!(
                            "Could not match value set for 'using' - '{s}' defaulting to Unknown"
                        );
                        DocumentType::Unknown
                    }
                },
                Err(e) => {
                    eprintln!("error: {e}");
                    warn!("An error occurred while reading the YAML file, defaulting to Unknown");
                    DocumentType::Unknown
                }
            },
        };
        info!("Detected {path} as {document_type}");
        match document_type {
            DocumentType::Unknown => (
                vec![],
                vec![
                    Box::new(JobsBlankLines::default()),
                    Box::new(StepsBlankLines::default()),
                    Box::new(TopLevelBlankLines::default()),
                    Box::new(TopLevelCommentSpacer::default()),
                    Box::new(VariableSpacer),
                ],
            ),
            DocumentType::Workflow => (
                vec![
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
                vec![
                    Box::new(JobsBlankLines::default()),
                    Box::new(StepsBlankLines::default()),
                    Box::new(TopLevelBlankLines::default()),
                    Box::new(TopLevelCommentSpacer::default()),
                    Box::new(VariableSpacer),
                ],
            ),
            DocumentType::CompositeAction => (
                vec![
                    Box::new(TopLevelSorter::new(
                        TOP_LEVEL_METADATA_KEY_ORDERING.map(String::from).to_vec(),
                    )),
                    Box::new(InputsSorter::default()),
                    Box::new(OutputsSorter::default()),
                    Box::new(RunsSorter::new(
                        COMPOSITE_KEY_ORDER.map(String::from).to_vec(),
                    )),
                    Box::new(StepSorter::default()),
                    Box::new(BrandingSorter::default()),
                    Box::new(CaseEnforcer::new(heck::ToSnakeCase::to_snake_case)),
                ],
                vec![
                    Box::new(JobsBlankLines::default()),
                    Box::new(StepsBlankLines::new(2)),
                    Box::new(TopLevelBlankLines::new(
                        TOP_LEVEL_METADATA_KEY_ORDERING.map(String::from).to_vec(),
                    )),
                    Box::new(TopLevelCommentSpacer::default()),
                    Box::new(VariableSpacer),
                ],
            ),
            DocumentType::DockerAction => (
                vec![
                    Box::new(TopLevelSorter::new(
                        TOP_LEVEL_METADATA_KEY_ORDERING.map(String::from).to_vec(),
                    )),
                    Box::new(InputsSorter::default()),
                    Box::new(OutputsSorter::default()),
                    Box::new(RunsSorter::new(DOCKER_KEY_ORDER.map(String::from).to_vec())),
                    Box::new(StepSorter::default()),
                    Box::new(BrandingSorter::default()),
                    Box::new(CaseEnforcer::new(heck::ToSnakeCase::to_snake_case)),
                ],
                vec![
                    Box::new(JobsBlankLines::default()),
                    Box::new(StepsBlankLines::new(2)),
                    Box::new(TopLevelBlankLines::new(
                        TOP_LEVEL_METADATA_KEY_ORDERING.map(String::from).to_vec(),
                    )),
                    Box::new(TopLevelCommentSpacer::default()),
                    Box::new(VariableSpacer),
                ],
            ),
            DocumentType::JavascriptAction => (
                vec![
                    Box::new(TopLevelSorter::new(
                        TOP_LEVEL_METADATA_KEY_ORDERING.map(String::from).to_vec(),
                    )),
                    Box::new(InputsSorter::default()),
                    Box::new(OutputsSorter::default()),
                    Box::new(RunsSorter::new(
                        JAVASCRIPT_KEY_ORDER.map(String::from).to_vec(),
                    )),
                    Box::new(StepSorter::default()),
                    Box::new(BrandingSorter::default()),
                    Box::new(CaseEnforcer::new(heck::ToSnakeCase::to_snake_case)),
                ],
                vec![
                    Box::new(JobsBlankLines::default()),
                    Box::new(StepsBlankLines::new(2)),
                    Box::new(TopLevelBlankLines::new(
                        TOP_LEVEL_METADATA_KEY_ORDERING.map(String::from).to_vec(),
                    )),
                    Box::new(TopLevelCommentSpacer::default()),
                    Box::new(VariableSpacer),
                ],
            ),
        }
    }
}
