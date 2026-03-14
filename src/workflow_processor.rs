//! Reads a workflow file, parses it, and applies the structure transformer pipeline.
use std::{fs, path::Path};

use fyaml::Document;
use tracing::{info, warn};

use crate::{
    errors::{Error, Result, Warning},
    structure_transformers::{
        CaseEnforcer, ConcurrencySorter, ContainerSorter, DefaultsSorter, EnvSorter,
        EnvironmentSorter, FilterSorter, JobSorter, NeedsSorter, OnSorter, PermissionsSorter,
        RunsOnSorter, StepSorter, StrategySorter, StructureTransformer, TopLevelSorter, WithSorter,
        WorkflowCallSorter, WorkflowDispatchSorter, WorkflowRunSorter,
    },
};

/// Applies the ordered sequence of [`StructureTransformer`]s to a parsed workflow document.
pub(crate) struct WorkflowProcessor {
    /// Ordered list of structure transformers to apply in sequence.
    transformers: Vec<Box<dyn StructureTransformer>>,
}

impl WorkflowProcessor {
    /// Create a `WorkflowProcessor` with the default transformer pipeline.
    pub(crate) fn new() -> Self {
        WorkflowProcessor {
            transformers: vec![
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
                Box::new(ConcurrencySorter),
                Box::new(EnvironmentSorter),
                Box::new(NeedsSorter::default()),
                Box::new(RunsOnSorter::default()),
                Box::new(FilterSorter::default()),
                Box::new(StrategySorter),
                Box::new(ContainerSorter::default()),
                Box::new(CaseEnforcer::new(heck::ToSnakeCase::to_snake_case)),
            ],
        }
    }

    /// Read and parse the workflow at `file`, apply all transformers, and return the result.
    ///
    /// Returns the transformed document alongside any non-fatal warnings produced by
    /// transformers that could not be applied. Fatal errors (unreadable file, invalid YAML)
    /// are returned as `Err`.
    pub(crate) fn process(&self, file: &Path) -> Result<(Document, Vec<Warning>)> {
        let content = fs::read_to_string(file).map_err(|source| Error::ReadFile {
            path: file.to_path_buf(),
            source,
        })?;
        self.process_str(content, &file.display().to_string())
    }

    /// Parse `content` (identified as `name` in diagnostics), apply all transformers,
    /// and return the result.
    ///
    /// Unlike [`process`][Self::process] this does not perform any I/O, making it
    /// suitable for formatting content that was not read from a file (e.g. stdin).
    pub(crate) fn process_str(
        &self,
        content: String,
        name: &str,
    ) -> Result<(Document, Vec<Warning>)> {
        let parse_result = Document::parse_str(&content);
        let mut document = parse_result.map_err(|e| Error::parse_yaml(name, content, &e))?;

        let mut warnings: Vec<Warning> = vec![];

        for transformer in &self.transformers {
            info!(
                "Applying structure transformer - {}",
                transformer.description()
            );
            // Snapshot the document as a YAML string before the call so we can restore
            // it if the transformer fails, allowing subsequent transformers to still run.
            let snapshot = document.to_string();
            match transformer.process(document) {
                Ok(doc) => document = doc,
                Err(e) => {
                    let transformer_name = transformer.name();
                    warn!("Transformer '{}' failed: {}", transformer_name, e);
                    let restore_result = Document::parse_str(&snapshot);
                    document = restore_result
                        .map_err(|e| Error::parse_yaml("<internal>", snapshot, &e))?;
                    warnings.push(Warning::StructureTransform {
                        transformer: transformer_name,
                        source: e,
                    });
                }
            }
        }

        Ok((document, warnings))
    }
}
