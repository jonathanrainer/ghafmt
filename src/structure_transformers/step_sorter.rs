//! Sorts the keys of each step within a job into idiomatic order.
use fyaml::Document;

use crate::structure_transformers::{
    StructureTransformer, for_each_mapping_child, for_each_seq_element,
};

/// Canonical key order for the top-level keys of each step.
const STEP_LEVEL_KEY_ORDERING: [&str; 11] = [
    "name",
    "id",
    "if",
    "uses",
    "run",
    "with",
    "env",
    "shell",
    "working-directory",
    "timeout-minutes",
    "continue-on-error",
];

/// Sorts the keys of each step into the canonical GHA ordering.
pub(crate) struct StepSorter {
    /// Pre-computed key ordering to avoid allocating on every call.
    key_ordering: Vec<String>,
}

impl Default for StepSorter {
    fn default() -> Self {
        Self {
            key_ordering: STEP_LEVEL_KEY_ORDERING.map(String::from).to_vec(),
        }
    }
}

impl StructureTransformer for StepSorter {
    fn process(&self, mut doc: Document) -> fyaml::Result<Document> {
        doc = for_each_mapping_child(doc, "/jobs", |doc, job_path| {
            for_each_seq_element(doc, &format!("{job_path}/steps"), |doc, step_path| {
                self.sort_mapping_at_path(doc, step_path, &self.key_ordering)
            })
        })?;
        Ok(doc)
    }

    fn name(&self) -> &'static str {
        "step-sorter"
    }

    fn description(&self) -> &'static str {
        "Sort 'steps' entries in an idiomatic way"
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
    #[case::no_steps(
        Document::from_string(indoc! {"
            jobs:
                build:
                    runs-on: ubuntu-latest
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              build:
                runs-on: ubuntu-latest
        "}.to_string()
    )]
    #[case::run_step_reordered(
        Document::from_string(indoc! {"
            jobs:
                build:
                    steps:
                        - run: echo hi
                          name: Greet
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              build:
                steps:
                - name: Greet
                  run: echo hi
        "}.to_string()
    )]
    #[case::uses_step_reordered(
        Document::from_string(indoc! {"
            jobs:
                build:
                    steps:
                        - with:
                              node-version: '20'
                          uses: actions/setup-node@v4
                          name: Setup Node
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              build:
                steps:
                - name: Setup Node
                  uses: actions/setup-node@v4
                  with:
                    node-version: '20'
        "}.to_string()
    )]
    #[case::full_step_keys_reordered(
        Document::from_string(indoc! {"
            jobs:
                build:
                    steps:
                        - continue-on-error: true
                          env:
                              CI: true
                          run: cargo test
                          id: tests
                          name: Run Tests
                          timeout-minutes: 10
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              build:
                steps:
                - name: Run Tests
                  id: tests
                  run: cargo test
                  env:
                    CI: true
                  timeout-minutes: 10
                  continue-on-error: true
        "}.to_string()
    )]
    #[case::multiple_steps_sorted(
        Document::from_string(indoc! {"
            jobs:
                build:
                    steps:
                        - run: echo hi
                          name: Greet
                        - with:
                              node-version: '20'
                          uses: actions/setup-node@v4
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              build:
                steps:
                - name: Greet
                  run: echo hi
                - uses: actions/setup-node@v4
                  with:
                    node-version: '20'
        "}.to_string()
    )]
    #[case::multiple_jobs_sorted(
        Document::from_string(indoc! {"
            jobs:
                build:
                    steps:
                        - run: echo build
                          name: Build
                test:
                    steps:
                        - run: cargo test
                          id: tests
                          name: Test
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              build:
                steps:
                - name: Build
                  run: echo build
              test:
                steps:
                - name: Test
                  id: tests
                  run: cargo test
        "}.to_string()
    )]
    #[case::already_ordered(
        Document::from_string(indoc! {"
            jobs:
                build:
                    steps:
                        - name: Greet
                          run: echo hi
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              build:
                steps:
                - name: Greet
                  run: echo hi
        "}.to_string()
    )]
    fn test_step_sorter(#[case] source_doc: Document, #[case] expected: String) {
        let result = StepSorter::default()
            .process(source_doc)
            .expect("processing failed")
            .to_string();

        assert_eq!(result, expected);
    }
}
