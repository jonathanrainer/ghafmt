//! Sorts `workflow_call` entries idiomatically and then alphabetically.

use fyaml::Document;

use crate::{constants::BRANDING_ORDER, structure_transformers::StructureTransformer};

/// Sorts `workflow_call` entries idiomatically and then alphabetically.
pub(crate) struct BrandingSorter {
    /// Pre-computed top-level key ordering to avoid allocating on every call.
    branding_key_order: Vec<String>,
}

impl Default for BrandingSorter {
    fn default() -> Self {
        Self {
            branding_key_order: BRANDING_ORDER.map(String::from).to_vec(),
        }
    }
}

impl StructureTransformer for BrandingSorter {
    fn process(&self, mut doc: Document) -> fyaml::Result<Document> {
        // Sort the entries under 'workflow_call' first
        doc = self.sort_mapping_at_path(doc, "/branding", &self.branding_key_order)?;
        Ok(doc)
    }

    fn name(&self) -> &'static str {
        "branding-sorter"
    }

    fn description(&self) -> &'static str {
        "Sort 'branding' entries idiomatically"
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
    #[case::no_branding(
        Document::from_string(indoc! {"
            name: My Action
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            name: My Action
        "}.to_string()
    )]
    #[case::color_before_icon(
        Document::from_string(indoc! {"
            branding:
                color: blue
                icon: star
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            branding:
              icon: star
              color: blue
        "}.to_string()
    )]
    #[case::already_ordered(
        Document::from_string(indoc! {"
            branding:
                icon: star
                color: blue
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            branding:
              icon: star
              color: blue
        "}.to_string()
    )]
    fn test_branding_sorter(#[case] source_doc: Document, #[case] expected: String) {
        let result = BrandingSorter::default()
            .process(source_doc)
            .expect("processing failed")
            .to_string();
        assert_eq!(result, expected);
    }
}
