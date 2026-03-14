//! Structure transformers that reorder or rename nodes in a parsed YAML workflow document.
mod case_enforcer;
mod concurrency_sorter;
mod container_sorter;
mod defaults_sorter;
mod env_sorter;
mod environment_sorter;
mod filter_sorter;
mod job_sorter;
mod needs_sorter;
mod on_sorter;
mod permissions_sorter;
mod runs_on_sorter;
mod step_sorter;
mod strategy_sorter;
mod top_level_sorter;
mod with_sorter;
mod workflow_call_sorter;
mod workflow_dispatch_sorter;
mod workflow_run_sorter;

use std::collections::HashMap;

pub(crate) use case_enforcer::CaseEnforcer;
pub(crate) use concurrency_sorter::ConcurrencySorter;
pub(crate) use container_sorter::ContainerSorter;
pub(crate) use defaults_sorter::DefaultsSorter;
pub(crate) use env_sorter::EnvSorter;
pub(crate) use environment_sorter::EnvironmentSorter;
pub(crate) use filter_sorter::FilterSorter;
use fyaml::Document;
pub(crate) use job_sorter::JobSorter;
pub(crate) use needs_sorter::NeedsSorter;
pub(crate) use on_sorter::OnSorter;
pub(crate) use permissions_sorter::PermissionsSorter;
pub(crate) use runs_on_sorter::RunsOnSorter;
pub(crate) use step_sorter::StepSorter;
pub(crate) use strategy_sorter::StrategySorter;
pub(crate) use top_level_sorter::TopLevelSorter;
use tracing::debug;
pub(crate) use with_sorter::WithSorter;
pub(crate) use workflow_call_sorter::WorkflowCallSorter;
pub(crate) use workflow_dispatch_sorter::WorkflowDispatchSorter;
pub(crate) use workflow_run_sorter::WorkflowRunSorter;

/// This trait captures what it means for a transform operation to act on the structure
/// of a YAML document. This can include things like sorting keys, or changing the names
/// of identifiers, anything related to the actual content of the YAML, rather than
/// how it is presented.
pub(crate) trait StructureTransformer {
    /// Apply this transformer to `doc` and return the (potentially reordered) result.
    fn process(&self, doc: Document) -> fyaml::Result<Document>;

    /// A short machine-friendly identifier for this transformer (e.g. `"job-sorter"`).
    /// Used in error messages and diagnostic codes.
    fn name(&self) -> &'static str;

    /// A human-readable description of what this transformer does.
    /// Used in log messages.
    fn description(&self) -> &'static str;

    /// Given a document, sort the mapping at the root path, applying the given ordering.
    /// If the root path isn't a mapping, then just return the document.
    #[allow(clippy::unwrap_used)]
    // scalar_str() returns Err if the node is not a scalar or contains non-UTF-8 bytes;
    // YAML map keys are always scalar nodes, and GHA workflows require valid UTF-8 throughout.
    fn sort_mapping_at_path(
        &self,
        mut workflow_document: Document,
        root_path: &str,
        key_ordering: &Vec<String>,
    ) -> fyaml::Result<Document> {
        if workflow_document
            .at_path(root_path)
            .is_some_and(|n| n.is_mapping())
        {
            debug!(
                "Sorting mapping at '{}', with order '{:?}'",
                root_path, key_ordering
            );
            let position_map: HashMap<&str, usize> = key_ordering
                .iter()
                .enumerate()
                .map(|(i, k)| (k.as_str(), i))
                .collect();
            let mut editor = workflow_document.edit();
            editor.sort_mapping_at(root_path, |k1, _, k2, _| {
                let s1 = k1.scalar_str().unwrap();
                let s2 = k2.scalar_str().unwrap();
                let pos1 = position_map.get(s1).copied().unwrap_or(usize::MAX);
                let pos2 = position_map.get(s2).copied().unwrap_or(usize::MAX);
                pos1.cmp(&pos2)
            })?;
        }

        Ok(workflow_document)
    }

    /// Helper method to allow sequences to be sorted alphabetically, given a specific document
    /// at a specific path
    #[allow(clippy::unwrap_used)]
    // scalar_str() returns Err if the node is not a scalar or contains non-UTF-8 bytes;
    // both are invariants that hold for any well-formed GHA workflow.
    fn sort_seq_at_path_alphabetically(
        &self,
        mut doc: Document,
        path: &str,
    ) -> fyaml::Result<Document> {
        if doc.at_path(path).is_some_and(|a| a.is_sequence()) {
            doc.edit().sort_sequence_at(path, |e1, e2| {
                let v1 = e1.scalar_str().unwrap();
                let v2 = e2.scalar_str().unwrap();
                v1.cmp(v2)
            })?;
        }
        Ok(doc)
    }

    /// Helper method to allow mappings to be sorted alphabetically, given a path and a specific
    /// document
    ///
    /// Returns the document gracefully if the path is not a mapping, and therefore cannot
    /// be sorted
    #[allow(clippy::unwrap_used)]
    // scalar_str() returns Err if the node is not a scalar or contains non-UTF-8 bytes;
    // both are invariants that hold for any well-formed GHA workflow.
    fn sort_path_to_mapping_alphabetically(
        &self,
        mut doc: Document,
        path: &str,
    ) -> fyaml::Result<Document> {
        if doc.at_path(path).is_some_and(|a| a.is_mapping()) {
            doc.edit().sort_mapping_at(path, |k1, _, k2, _| {
                let key1 = k1.scalar_str().unwrap();
                let key2 = k2.scalar_str().unwrap();
                key1.cmp(key2)
            })?;
        }
        Ok(doc)
    }
}

/// A further helper function that applies a closure to each of the children of a parent path
/// in a document that is a mapping.
///
/// Exits gracefully, and returns the unedited document, if the path given is not a mapping
fn for_each_mapping_child<F>(
    mut doc: Document,
    parent_path: &str,
    mut f: F,
) -> fyaml::Result<Document>
where
    F: FnMut(Document, &str) -> fyaml::Result<Document>,
{
    let keys: Vec<String> = doc.at_path(parent_path).map_or(Ok(vec![]), |n| {
        n.map_iter()
            .map(|(k, _)| k.scalar_str().map(String::from))
            .collect::<fyaml::Result<_>>()
    })?;
    for key in keys {
        let path = &format!("{parent_path}/{key}");
        debug!("Applying function to {}", path);
        doc = f(doc, path)?;
    }
    Ok(doc)
}

/// A further helper function that applies a closure to each of the children of a parent path
/// in a document that is a sequence.
///
/// Exits gracefully, and returns the unedited document, if the path given is not a sequence
fn for_each_seq_element<F>(mut doc: Document, seq_path: &str, mut f: F) -> fyaml::Result<Document>
where
    F: FnMut(Document, &str) -> fyaml::Result<Document>,
{
    let len = doc.at_path(seq_path).map_or(Ok(0), |n| n.seq_len())?;
    debug!("Applying function to each element of {}", seq_path);
    for i in 0..len {
        doc = f(doc, &format!("{seq_path}/{i}"))?;
    }
    Ok(doc)
}
