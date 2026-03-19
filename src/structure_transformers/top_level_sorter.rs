//! Sorts the top-level keys of a GitHub Actions workflow into idiomatic order.
use fyaml::Document;

use crate::{constants::TOP_LEVEL_KEY_ORDERING, structure_transformers::StructureTransformer};

/// Sorts the top-level workflow keys into the canonical GHA ordering.
pub(crate) struct TopLevelSorter {
    /// Pre-computed key ordering to avoid allocating on every call.
    key_ordering: Vec<String>,
}

impl Default for TopLevelSorter {
    fn default() -> Self {
        Self {
            key_ordering: TOP_LEVEL_KEY_ORDERING.map(String::from).to_vec(),
        }
    }
}

impl StructureTransformer for TopLevelSorter {
    fn process(&self, doc: Document) -> fyaml::Result<Document> {
        self.sort_mapping_at_path(doc, "", &self.key_ordering)
    }

    fn name(&self) -> &'static str {
        "top-level-sorter"
    }

    fn description(&self) -> &'static str {
        "Sort top-level entries in an idiomatic way"
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
    #[case::minimal_workflow(
        Document::from_string(indoc! {"
            jobs:
                build:
                    runs-on: ubuntu-latest
            on: push
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            on: push
            jobs:
              build:
                runs-on: ubuntu-latest
        "}.to_string()
    )]
    #[case::all_keys_reordered(
        Document::from_string(indoc! {"
            jobs:
                build:
                    runs-on: ubuntu-latest
            defaults:
                run:
                    shell: bash
            env:
                CI: true
            concurrency: my-group
            permissions:
                contents: read
            on: push
            run-name: My Run
            name: My Workflow
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            name: My Workflow
            run-name: My Run
            on: push
            permissions:
              contents: read
            concurrency: my-group
            env:
              CI: true
            defaults:
              run:
                shell: bash
            jobs:
              build:
                runs-on: ubuntu-latest
        "}.to_string()
    )]
    #[case::already_ordered(
        Document::from_string(indoc! {"
            name: My Workflow
            on: push
            jobs:
                build:
                    runs-on: ubuntu-latest
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            name: My Workflow
            on: push
            jobs:
              build:
                runs-on: ubuntu-latest
        "}.to_string()
    )]
    fn test_top_level_sorter(#[case] source_doc: Document, #[case] expected: String) {
        let result = TopLevelSorter::default()
            .process(source_doc)
            .expect("processing failed")
            .to_string();

        assert_eq!(result, expected);
    }
}
