//! Reads a workflow file, parses it, and applies the structure transformer pipeline.
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
    /// Create a `WorkflowProcessor` with a custom transformer pipeline.
    pub(crate) fn new(transformers: Vec<Box<dyn StructureTransformer>>) -> Self {
        WorkflowProcessor { transformers }
    }

    /// Parse `content` (identified as `name` in diagnostics), apply all transformers,
    /// and return the result.
    pub(crate) fn process(&self, content: &str, name: &str) -> Result<(Document, Vec<Warning>)> {
        let parse_result = Document::parse_str(content);
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
                        .map_err(|e| Error::parse_yaml("<internal>", &snapshot, &e))?;
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

impl Default for WorkflowProcessor {
    fn default() -> Self {
        Self::new(vec![
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
        ])
    }
}

#[cfg(test)]
mod tests {
    use fyaml::Document;

    use super::*;

    /// A transformer that always returns an error, used to test error-recovery.
    struct AlwaysFail;

    impl StructureTransformer for AlwaysFail {
        fn process(&self, _doc: Document) -> fyaml::Result<Document> {
            Err(fyaml::Error::Ffi("injected failure"))
        }

        fn name(&self) -> &'static str {
            "always-fail"
        }

        fn description(&self) -> &'static str {
            "Always fails"
        }
    }

    /// A transformer that renames the first top-level key to a known value,
    /// proving it ran by leaving a detectable mark on the document.
    struct AppendMarker {
        /// A key to append at the top level (value: "marker").
        key: &'static str,
    }

    impl StructureTransformer for AppendMarker {
        fn process(&self, mut doc: Document) -> fyaml::Result<Document> {
            doc.edit().set_yaml_at(self.key, "marker")?;
            Ok(doc)
        }

        fn name(&self) -> &'static str {
            "append-marker"
        }

        fn description(&self) -> &'static str {
            "Appends a marker key"
        }
    }

    #[test]
    fn failed_transformer_produces_warning() {
        let proc = WorkflowProcessor::new(vec![Box::new(AlwaysFail)]);
        let (_, warnings) = proc.process("a: b\n", "test").expect("process failed");
        assert_eq!(warnings.len(), 1);
        assert!(
            matches!(&warnings[0], Warning::StructureTransform { transformer, .. } if *transformer == "always-fail")
        );
    }

    #[test]
    fn failed_transformer_document_is_restored() {
        // AlwaysFail runs but fails; the document should be unchanged after it.
        let proc = WorkflowProcessor::new(vec![Box::new(AlwaysFail)]);
        let (doc, _) = proc.process("a: b\n", "test").expect("process failed");
        assert_eq!(doc.to_string(), "a: b\n");
    }

    #[test]
    fn subsequent_transformers_run_after_failure() {
        // Pipeline: mark with "before", fail, mark with "after".
        // Both markers should be present in the output.
        let proc = WorkflowProcessor::new(vec![
            Box::new(AppendMarker { key: "before" }),
            Box::new(AlwaysFail),
            Box::new(AppendMarker { key: "after" }),
        ]);
        let (doc, warnings) = proc.process("a: b\n", "test").expect("process failed");
        let output = doc.to_string();
        assert_eq!(warnings.len(), 1);
        assert!(output.contains("before"), "expected 'before' in: {output}");
        assert!(output.contains("after"), "expected 'after' in: {output}");
    }
}
