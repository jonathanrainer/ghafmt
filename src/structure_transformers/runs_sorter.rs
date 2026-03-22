//! Sorts keys under `container` and `services` within a job into idiomatic order.
use fyaml::Document;

use crate::{
    constants::STEP_LEVEL_KEY_ORDERING,
    structure_transformers::{StructureTransformer, for_each_seq_element},
};

/// Sorts keys under `runs` into an idiomatic order for the given action type.
pub(crate) struct RunsSorter {
    /// Pre-computed key ordering to avoid allocating on every call.
    key_ordering: Vec<String>,
}

impl RunsSorter {
    /// Creates a `RunsSorter` with the given key ordering.
    pub(crate) fn new(key_ordering: Vec<String>) -> Self {
        Self { key_ordering }
    }
}

impl StructureTransformer for RunsSorter {
    fn process(&self, mut doc: Document) -> fyaml::Result<Document> {
        // Sort the keys at the top level first
        doc = self.sort_mapping_at_path(doc, "/runs", &self.key_ordering)?;
        doc = self.sort_path_to_mapping_alphabetically(doc, "/runs/args")?;
        doc = for_each_seq_element(doc, "/runs/steps", |mut document, parent_path| {
            document = self.sort_mapping_at_path(
                document,
                parent_path,
                STEP_LEVEL_KEY_ORDERING.map(String::from).as_ref(),
            )?;
            self.sort_path_to_mapping_alphabetically(document, &format!("{parent_path}/with"))
        })?;
        doc = self.sort_path_to_mapping_alphabetically(doc, "/runs/with")?;
        Ok(doc)
    }

    fn name(&self) -> &'static str {
        "runs-sorter"
    }

    fn description(&self) -> &'static str {
        "Sort keys under 'runs' into an idiomatic order"
    }
}

#[cfg(test)]
mod tests {
    use fyaml::Document;
    use indoc::indoc;
    use rstest::rstest;
    use similar_asserts::assert_eq;

    use super::*;

    fn composite_sorter() -> RunsSorter {
        RunsSorter::new(vec!["using".into(), "steps".into()])
    }

    fn javascript_sorter() -> RunsSorter {
        RunsSorter::new(vec![
            "using".into(),
            "pre".into(),
            "pre-if".into(),
            "main".into(),
            "post".into(),
            "post-if".into(),
        ])
    }

    fn docker_sorter() -> RunsSorter {
        RunsSorter::new(vec![
            "using".into(),
            "image".into(),
            "args".into(),
            "env".into(),
            "pre-entrypoint".into(),
            "entrypoint".into(),
            "post-entrypoint".into(),
        ])
    }

    #[rstest]
    #[case::no_runs(
        Document::from_string(indoc! {"
            name: My Action
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            name: My Action
        "}.to_string()
    )]
    #[case::steps_before_using(
        Document::from_string(indoc! {"
            runs:
                steps:
                    - run: echo hello
                      name: Build
                using: composite
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            runs:
              using: composite
              steps:
              - name: Build
                run: echo hello
        "}.to_string()
    )]
    #[case::already_ordered(
        Document::from_string(indoc! {"
            runs:
                using: composite
                steps:
                    - name: Build
                      run: echo hello
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            runs:
              using: composite
              steps:
              - name: Build
                run: echo hello
        "}.to_string()
    )]
    fn test_composite_runs_sorter(#[case] source_doc: Document, #[case] expected: String) {
        let result = composite_sorter()
            .process(source_doc)
            .expect("processing failed")
            .to_string();
        assert_eq!(result, expected);
    }

    #[rstest]
    #[case::runs_reordered(
        Document::from_string(indoc! {"
            runs:
                main: dist/index.js
                post: dist/cleanup.js
                using: node20
                pre: dist/setup.js
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            runs:
              using: node20
              pre: dist/setup.js
              main: dist/index.js
              post: dist/cleanup.js
        "}.to_string()
    )]
    #[case::already_ordered(
        Document::from_string(indoc! {"
            runs:
                using: node20
                main: dist/index.js
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            runs:
              using: node20
              main: dist/index.js
        "}.to_string()
    )]
    fn test_javascript_runs_sorter(#[case] source_doc: Document, #[case] expected: String) {
        let result = javascript_sorter()
            .process(source_doc)
            .expect("processing failed")
            .to_string();
        assert_eq!(result, expected);
    }

    #[rstest]
    #[case::runs_reordered(
        Document::from_string(indoc! {"
            runs:
                entrypoint: entrypoint.sh
                using: docker
                image: Dockerfile
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            runs:
              using: docker
              image: Dockerfile
              entrypoint: entrypoint.sh
        "}.to_string()
    )]
    #[case::with_keys_sorted_alphabetically(
        Document::from_string(indoc! {"
            runs:
                using: composite
                steps:
                    - uses: actions/checkout@v4
                      with:
                          token: abc
                          depth: '1'
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            runs:
              using: composite
              steps:
              - uses: actions/checkout@v4
                with:
                  depth: '1'
                  token: abc
        "}.to_string()
    )]
    fn test_docker_runs_sorter(#[case] source_doc: Document, #[case] expected: String) {
        let result = docker_sorter()
            .process(source_doc)
            .expect("processing failed")
            .to_string();
        assert_eq!(result, expected);
    }
}
