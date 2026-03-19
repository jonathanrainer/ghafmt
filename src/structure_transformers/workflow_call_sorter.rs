//! Sorts `workflow_call` entries idiomatically and then alphabetically.
use std::collections::HashMap;

use fyaml::Document;

use crate::{
    constants::{INPUT_ORDER, OUTPUT_ORDER, SECRET_ORDER, WORKFLOW_KEYS},
    structure_transformers::{StructureTransformer, for_each_mapping_child},
};

/// Sorts `workflow_call` entries idiomatically and then alphabetically.
pub(crate) struct WorkflowCallSorter {
    /// Pre-computed top-level key ordering to avoid allocating on every call.
    workflow_key_order: Vec<String>,
    /// A map that defines a key that can be found under "`workflow_call`" and the
    /// order its sub-keys should have.
    order_map: HashMap<String, Vec<String>>,
}

impl Default for WorkflowCallSorter {
    fn default() -> Self {
        WorkflowCallSorter {
            workflow_key_order: WORKFLOW_KEYS.map(String::from).to_vec(),
            order_map: HashMap::from([
                ("inputs".to_string(), INPUT_ORDER.map(String::from).to_vec()),
                (
                    "outputs".to_string(),
                    OUTPUT_ORDER.map(String::from).to_vec(),
                ),
                (
                    "secrets".to_string(),
                    SECRET_ORDER.map(String::from).to_vec(),
                ),
            ]),
        }
    }
}

impl StructureTransformer for WorkflowCallSorter {
    fn process(&self, mut doc: Document) -> fyaml::Result<Document> {
        // Sort the entries under 'workflow_call' first
        doc = self.sort_mapping_at_path(doc, "/on/workflow_call", &self.workflow_key_order)?;
        // Then for each of the keys under 'workflow_call' sort their children alphabetically
        doc = for_each_mapping_child(doc, "/on/workflow_call", |doc, child_path| {
            self.sort_path_to_mapping_alphabetically(doc, child_path)
        })?;
        // For a specific set of keys, defined in the order_map, re-order the keys to the order
        // specified in the order_map.
        for (key, key_order) in &self.order_map {
            doc = for_each_mapping_child(
                doc,
                &format!("/on/workflow_call/{key}"),
                |doc, child_path| self.sort_mapping_at_path(doc, child_path, key_order),
            )?;
        }
        Ok(doc)
    }

    fn name(&self) -> &'static str {
        "workflow-call-sorter"
    }

    fn description(&self) -> &'static str {
        "Sort 'workflow_call' entries first idiomatically and then alphabetically"
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
    #[case::no_workflow_call(
        Document::from_string(indoc! {"
            on: push
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            on: push
        "}.to_string()
    )]
    #[case::sections_sorted(
        Document::from_string(indoc! {"
            on:
                workflow_call:
                    secrets:
                        deploy_token:
                            required: true
                    outputs:
                        deploy_url:
                            description: The URL
                            value: result
                    inputs:
                        env:
                            type: string
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            on:
              workflow_call:
                inputs:
                  env:
                    type: string
                outputs:
                  deploy_url:
                    description: The URL
                    value: result
                secrets:
                  deploy_token:
                    required: true
        "}.to_string()
    )]
    #[case::input_entries_sorted_alphabetically(
        Document::from_string(indoc! {"
            on:
                workflow_call:
                    inputs:
                        version:
                            type: string
                        environment:
                            type: string
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            on:
              workflow_call:
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
                workflow_call:
                    inputs:
                        env:
                            required: true
                            default: staging
                            type: string
                            description: Target environment
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            on:
              workflow_call:
                inputs:
                  env:
                    description: Target environment
                    type: string
                    required: true
                    default: staging
        "}.to_string()
    )]
    #[case::output_entries_sorted_alphabetically(
        Document::from_string(indoc! {"
            on:
                workflow_call:
                    outputs:
                        version:
                            description: Version
                            value: v1
                        deploy_url:
                            description: The URL
                            value: https://example.com
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            on:
              workflow_call:
                outputs:
                  deploy_url:
                    description: The URL
                    value: https://example.com
                  version:
                    description: Version
                    value: v1
        "}.to_string()
    )]
    #[case::output_keys_sorted(
        Document::from_string(indoc! {"
            on:
                workflow_call:
                    outputs:
                        deploy_url:
                            value: https://example.com
                            description: The URL
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            on:
              workflow_call:
                outputs:
                  deploy_url:
                    description: The URL
                    value: https://example.com
        "}.to_string()
    )]
    #[case::secret_entries_sorted_alphabetically(
        Document::from_string(indoc! {"
            on:
                workflow_call:
                    secrets:
                        slack_webhook:
                            required: false
                        deploy_token:
                            required: true
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            on:
              workflow_call:
                secrets:
                  deploy_token:
                    required: true
                  slack_webhook:
                    required: false
        "}.to_string()
    )]
    #[case::secret_keys_sorted(
        Document::from_string(indoc! {"
            on:
                workflow_call:
                    secrets:
                        deploy_token:
                            required: true
                            description: Token for deployment
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            on:
              workflow_call:
                secrets:
                  deploy_token:
                    description: Token for deployment
                    required: true
        "}.to_string()
    )]
    fn test_workflow_call_sorter(#[case] source_doc: Document, #[case] expected: String) {
        let result = WorkflowCallSorter::default()
            .process(source_doc)
            .expect("processing failed")
            .to_string();

        assert_eq!(result, expected);
    }
}
