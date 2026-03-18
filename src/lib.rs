//! Library entry point for `ghafmt`.
//!
//! Exposes [`Ghafmt`], which orchestrates the structure-transformation and
//! presentation-transformation pipeline for GitHub Actions workflow files.
use std::path::Path;

use tracing::info;

use crate::{workflow_emitter::WorkflowEmitter, workflow_processor::WorkflowProcessor};

mod constants;
pub mod errors;
pub use errors::{Error, Result, Warning};
mod presentation_transformers;
mod structure_transformers;
mod workflow_emitter;
mod workflow_processor;

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

    /// Format the GitHub Actions workflow at `file` and return the formatted YAML string
    /// along with any non-fatal warnings produced during processing.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or cannot be parsed as YAML.
    /// Transformer failures are returned as warnings rather than errors.
    pub fn format_gha_workflow(&mut self, file: &Path) -> Result<(String, Vec<Warning>)> {
        info!("Beginning Document Processing...");
        let (yaml, warnings) = self.workflow_processor.process(file)?;
        info!("Emitting workflow...");
        let yaml_string = self.workflow_emitter.emit(&yaml)?;
        Ok((yaml_string, warnings))
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
    pub fn format_str(&mut self, content: &str, name: &str) -> Result<(String, Vec<Warning>)> {
        info!("Beginning Document Processing...");
        let (yaml, warnings) = self.workflow_processor.process_str(content, name)?;
        info!("Emitting workflow...");
        let yaml_string = self.workflow_emitter.emit(&yaml)?;
        Ok((yaml_string, warnings))
    }
}
