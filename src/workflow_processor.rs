//! Reads a workflow file, parses it, and applies the structure transformer pipeline.
use fyaml::Document;
use tracing::{info, warn};

use crate::{
    errors::{Error, Result, Warning},
    structure_transformers::StructureTransformer,
};

/// Applies the ordered sequence of [`StructureTransformer`]s to a parsed workflow document.
pub(crate) struct Processor {
    /// Ordered list of structure transformers to apply in sequence.
    transformers: Vec<Box<dyn StructureTransformer>>,
}

impl Processor {
    /// Create a `WorkflowProcessor` with a custom transformer pipeline.
    pub(crate) fn new(transformers: Vec<Box<dyn StructureTransformer>>) -> Self {
        Processor { transformers }
    }

    /// Parse `content` (identified as `name` in diagnostics), apply all transformers,
    /// and return the result.
    pub(crate) fn process(&self, mut document: Document) -> Result<(Document, Vec<Warning>)> {
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
        let proc = Processor::new(vec![Box::new(AlwaysFail)]);
        let starter_doc = Document::from_string("a: b\n".to_string()).expect("valid YAML");
        let (_, warnings) = proc.process(starter_doc).expect("process failed");
        assert_eq!(warnings.len(), 1);
        assert!(
            matches!(&warnings[0], Warning::StructureTransform { transformer, .. } if *transformer == "always-fail")
        );
    }

    #[test]
    fn failed_transformer_document_is_restored() {
        // AlwaysFail runs but fails; the document should be unchanged after it.
        let proc = Processor::new(vec![Box::new(AlwaysFail)]);
        let starter_doc = Document::from_string("a: b\n".to_string()).expect("valid YAML");
        let (doc, _) = proc.process(starter_doc).expect("process failed");
        assert_eq!(doc.to_string(), "a: b\n");
    }

    #[test]
    fn subsequent_transformers_run_after_failure() {
        // Pipeline: mark with "before", fail, mark with "after".
        // Both markers should be present in the output.
        let proc = Processor::new(vec![
            Box::new(AppendMarker { key: "before" }),
            Box::new(AlwaysFail),
            Box::new(AppendMarker { key: "after" }),
        ]);
        let starter_doc = Document::from_string("a: b\n".to_string()).expect("valid YAML");
        let (doc, warnings) = proc.process(starter_doc).expect("process failed");
        let output = doc.to_string();
        assert_eq!(warnings.len(), 1);
        assert!(output.contains("before"), "expected 'before' in: {output}");
        assert!(output.contains("after"), "expected 'after' in: {output}");
    }
}
