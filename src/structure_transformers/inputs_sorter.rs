//! Sorts `workflow_call` entries idiomatically and then alphabetically.

use fyaml::Document;

use crate::{
    constants::INPUT_ORDER,
    structure_transformers::{for_each_mapping_child, StructureTransformer},
};

/// Sorts `workflow_call` entries idiomatically and then alphabetically.
pub(crate) struct InputsSorter {
    /// Pre-computed top-level key ordering to avoid allocating on every call.
    inputs_key_order: Vec<String>,
}

impl Default for InputsSorter {
    fn default() -> Self {
        Self {
            inputs_key_order: INPUT_ORDER.map(String::from).to_vec(),
        }
    }
}

impl StructureTransformer for InputsSorter {
    fn process(&self, mut doc: Document) -> fyaml::Result<Document> {
        doc = self.sort_path_to_mapping_alphabetically(doc, "/inputs")?;
        doc = for_each_mapping_child(doc, "/inputs", |doc, child_path| {
            self.sort_mapping_at_path(doc, child_path, &self.inputs_key_order)
        })?;
        Ok(doc)
    }

    fn name(&self) -> &'static str {
        "inputs-sorter"
    }

    fn description(&self) -> &'static str {
        "Sort 'inputs' entries first alphabetically, then idiomatically"
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
    #[case::no_inputs(
        Document::from_string(indoc! {"
            name: My Action
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            name: My Action
        "}.to_string()
    )]
    #[case::inputs_alpha_sorted(
        Document::from_string(indoc! {"
            inputs:
                zebra:
                    description: A zebra
                apple:
                    description: An apple
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            inputs:
              apple:
                description: An apple
              zebra:
                description: A zebra
        "}.to_string()
    )]
    #[case::input_entry_keys_sorted(
        Document::from_string(indoc! {"
            inputs:
                greeting:
                    required: true
                    default: hello
                    description: A greeting
                    type: string
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            inputs:
              greeting:
                description: A greeting
                type: string
                required: true
                default: hello
        "}.to_string()
    )]
    #[case::alpha_and_entry_keys_both_sorted(
        Document::from_string(indoc! {"
            inputs:
                zebra:
                    required: false
                    description: A zebra input
                apple:
                    required: true
                    description: An apple input
                    type: string
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            inputs:
              apple:
                description: An apple input
                type: string
                required: true
              zebra:
                description: A zebra input
                required: false
        "}.to_string()
    )]
    #[case::already_sorted(
        Document::from_string(indoc! {"
            inputs:
                apple:
                    description: An apple
                    required: true
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            inputs:
              apple:
                description: An apple
                required: true
        "}.to_string()
    )]
    fn test_inputs_sorter(#[case] source_doc: Document, #[case] expected: String) {
        let result = InputsSorter::default()
            .process(source_doc)
            .expect("processing failed")
            .to_string();
        assert_eq!(result, expected);
    }
}
