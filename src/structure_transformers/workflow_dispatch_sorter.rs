//! Sorts `workflow_dispatch` entries idiomatically and then alphabetically.
use std::collections::HashMap;

use fyaml::Document;

use crate::{
    constants::INPUT_ORDER,
    structure_transformers::{StructureTransformer, for_each_mapping_child},
};

/// Sorts `workflow_dispatch` entries idiomatically and then alphabetically.
pub(crate) struct WorkflowDispatchSorter {
    /// A map that defines a key that can be found under "`workflow_dispatch`" and the
    /// order its sub-keys should have.
    order_map: HashMap<String, Vec<String>>,
}

impl Default for WorkflowDispatchSorter {
    fn default() -> Self {
        WorkflowDispatchSorter {
            order_map: HashMap::from([(
                "inputs".to_string(),
                INPUT_ORDER.map(String::from).to_vec(),
            )]),
        }
    }
}

impl StructureTransformer for WorkflowDispatchSorter {
    fn process(&self, mut doc: Document) -> fyaml::Result<Document> {
        doc = self.sort_path_to_mapping_alphabetically(doc, "/on/workflow_dispatch")?;
        doc = for_each_mapping_child(doc, "/on/workflow_dispatch", |doc, child_path| {
            self.sort_path_to_mapping_alphabetically(doc, child_path)
        })?;
        for (key, key_order) in &self.order_map {
            doc = for_each_mapping_child(
                doc,
                &format!("/on/workflow_dispatch/{key}"),
                |doc, child_path| self.sort_mapping_at_path(doc, child_path, key_order),
            )?;
        }
        Ok(doc)
    }

    fn name(&self) -> &'static str {
        "workflow-dispatch-sorter"
    }

    fn description(&self) -> &'static str {
        "Sort 'workflow_dispatch' entries first idiomatically and then alphabetically"
    }
}

#[cfg(test)]
mod tests {
    use fyaml::Document;
    use indoc::indoc;
    use rstest::rstest;
    use similar_asserts::assert_eq;

    use super::*;

    #[rstest]
    #[case::no_workflow_dispatch(
        Document::from_string(indoc! {"
            on: push
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            on: push
        "}.to_string()
    )]
    #[case::input_entries_sorted_alphabetically(
        Document::from_string(indoc! {"
            on:
                workflow_dispatch:
                    inputs:
                        version:
                            type: string
                        environment:
                            type: string
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            on:
              workflow_dispatch:
                inputs:
                  environment:
                    type: string
                  version:
                    type: string
        "}.to_string()
    )]
    #[case::input_keys_sorted(
        Document::from_string(indoc! {"
            on:
                workflow_dispatch:
                    inputs:
                        env:
                            required: true
                            default: staging
                            type: choice
                            description: Target environment
                            options:
                                - staging
                                - production
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            on:
              workflow_dispatch:
                inputs:
                  env:
                    description: Target environment
                    type: choice
                    required: true
                    default: staging
                    options:
                    - staging
                    - production
        "}.to_string()
    )]
    #[case::already_sorted(
        Document::from_string(indoc! {"
            on:
                workflow_dispatch:
                    inputs:
                        debug:
                            description: Enable debug
                            type: boolean
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            on:
              workflow_dispatch:
                inputs:
                  debug:
                    description: Enable debug
                    type: boolean
        "}.to_string()
    )]
    fn test_workflow_dispatch_sorter(#[case] source_doc: Document, #[case] expected: String) {
        let result = WorkflowDispatchSorter::default()
            .process(source_doc)
            .expect("processing failed")
            .to_string();

        assert_eq!(result, expected);
    }
}
