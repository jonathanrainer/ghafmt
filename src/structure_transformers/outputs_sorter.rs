//! Sorts `workflow_call` entries idiomatically and then alphabetically.

use fyaml::Document;

use crate::{
    constants::OUTPUT_ORDER,
    structure_transformers::{for_each_mapping_child, StructureTransformer},
};

/// Sorts `workflow_call` entries idiomatically and then alphabetically.
pub(crate) struct OutputsSorter {
    /// Pre-computed top-level key ordering to avoid allocating on every call.
    outputs_key_order: Vec<String>,
}

impl Default for OutputsSorter {
    fn default() -> Self {
        Self {
            outputs_key_order: OUTPUT_ORDER.map(String::from).to_vec(),
        }
    }
}

impl StructureTransformer for OutputsSorter {
    fn process(&self, mut doc: Document) -> fyaml::Result<Document> {
        doc = self.sort_path_to_mapping_alphabetically(doc, "/outputs")?;
        doc = for_each_mapping_child(doc, "/outputs", |doc, child_path| {
            self.sort_mapping_at_path(doc, child_path, &self.outputs_key_order)
        })?;
        Ok(doc)
    }

    fn name(&self) -> &'static str {
        "outputs-sorter"
    }

    fn description(&self) -> &'static str {
        "Sort 'outputs' entries first alphabetically, then idiomatically"
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
    #[case::no_outputs(
        Document::from_string(indoc! {"
            name: My Action
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            name: My Action
        "}.to_string()
    )]
    #[case::outputs_alpha_sorted(
        Document::from_string(indoc! {"
            outputs:
                zoo:
                    description: Zoo output
                    value: zoo_val
                ant:
                    description: Ant output
                    value: ant_val
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            outputs:
              ant:
                description: Ant output
                value: ant_val
              zoo:
                description: Zoo output
                value: zoo_val
        "}.to_string()
    )]
    #[case::output_entry_keys_sorted(
        Document::from_string(indoc! {"
            outputs:
                result:
                    value: some_val
                    description: The result
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            outputs:
              result:
                description: The result
                value: some_val
        "}.to_string()
    )]
    #[case::alpha_and_entry_keys_both_sorted(
        Document::from_string(indoc! {"
            outputs:
                zoo:
                    value: zoo_val
                    description: Zoo output
                ant:
                    value: ant_val
                    description: Ant output
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            outputs:
              ant:
                description: Ant output
                value: ant_val
              zoo:
                description: Zoo output
                value: zoo_val
        "}.to_string()
    )]
    #[case::already_sorted(
        Document::from_string(indoc! {"
            outputs:
                result:
                    description: The result
                    value: some_val
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            outputs:
              result:
                description: The result
                value: some_val
        "}.to_string()
    )]
    fn test_outputs_sorter(#[case] source_doc: Document, #[case] expected: String) {
        let result = OutputsSorter::default()
            .process(source_doc)
            .expect("processing failed")
            .to_string();
        assert_eq!(result, expected);
    }
}
