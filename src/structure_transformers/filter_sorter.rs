//! Sorts all filter entries (branches, tags, paths, types) alphabetically.
use fyaml::Document;

use crate::{constants::EVENT_TYPES, structure_transformers::StructureTransformer};

/// Sorts all filter entries (branches, tags, paths, types) alphabetically.
pub(crate) struct FilterSorter {
    /// YPATH-style locations of sequences to sort alphabetically.
    filter_locations: Vec<String>,
}

impl Default for FilterSorter {
    fn default() -> FilterSorter {
        let mut filter_locations: Vec<String> = vec![
            "on/pull_request/branches",
            "on/pull_request/branches-ignore",
            "on/pull_request_target/branches",
            "on/pull_request_target/branches-ignore",
            "on/push/branches",
            "on/push/branches-ignore",
            "on/push/tags",
            "on/push/tags-ignore",
            "on/push/paths",
            "on/push/paths-ignore",
            "on/pull_request/paths",
            "on/pull_request/paths-ignore",
            "on/pull_request_target/paths",
            "on/pull_request_target/paths-ignore",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        filter_locations.append(&mut EVENT_TYPES.map(|a| format!("on/{a}/types")).to_vec());

        Self { filter_locations }
    }
}

impl StructureTransformer for FilterSorter {
    fn process(&self, mut doc: Document) -> fyaml::Result<Document> {
        for filter_location in &self.filter_locations {
            doc = self.sort_seq_at_path_alphabetically(doc, filter_location)?;
        }
        Ok(doc)
    }

    fn name(&self) -> &'static str {
        "filter-sorter"
    }

    fn description(&self) -> &'static str {
        "Sort all the filter entries"
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
    #[case::no_filter_keys(
        Document::from_string(indoc! {"
            on: push
            jobs:
                build:
                    runs-on: ubuntu-latest
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            on: push
            jobs:
              build:
                runs-on: ubuntu-latest
        "}.to_string()
    )]
    #[case::push_branches_sorted(
        Document::from_string(indoc! {"
            on:
                push:
                    branches:
                        - main
                        - develop
                        - feature/foo
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            on:
              push:
                branches:
                - develop
                - feature/foo
                - main
        "}.to_string()
    )]
    #[case::push_branches_ignore_sorted(
        Document::from_string(indoc! {"
            on:
                push:
                    branches-ignore:
                        - main
                        - develop
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            on:
              push:
                branches-ignore:
                - develop
                - main
        "}.to_string()
    )]
    #[case::push_tags_sorted(
        Document::from_string(indoc! {"
            on:
                push:
                    tags:
                        - 'v2.*'
                        - 'v1.*'
                        - 'v1.0.0'
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            on:
              push:
                tags:
                - 'v1.*'
                - 'v1.0.0'
                - 'v2.*'
        "}.to_string()
    )]
    #[case::push_tags_ignore_sorted(
        Document::from_string(indoc! {"
            on:
                push:
                    tags-ignore:
                        - 'v2.*'
                        - 'v1.*'
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            on:
              push:
                tags-ignore:
                - 'v1.*'
                - 'v2.*'
        "}.to_string()
    )]
    #[case::push_paths_sorted(
        Document::from_string(indoc! {"
            on:
                push:
                    paths:
                        - 'tests/**'
                        - 'src/**'
                        - '*.json'
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            on:
              push:
                paths:
                - '*.json'
                - 'src/**'
                - 'tests/**'
        "}.to_string()
    )]
    #[case::push_paths_ignore_sorted(
        Document::from_string(indoc! {"
            on:
                push:
                    paths-ignore:
                        - 'docs/**'
                        - '*.md'
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            on:
              push:
                paths-ignore:
                - '*.md'
                - 'docs/**'
        "}.to_string()
    )]
    #[case::pull_request_branches_sorted(
        Document::from_string(indoc! {"
            on:
                pull_request:
                    branches:
                        - main
                        - develop
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            on:
              pull_request:
                branches:
                - develop
                - main
        "}.to_string()
    )]
    #[case::pull_request_types_sorted(
        Document::from_string(indoc! {"
            on:
                pull_request:
                    types:
                        - synchronize
                        - opened
                        - reopened
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            on:
              pull_request:
                types:
                - opened
                - reopened
                - synchronize
        "}.to_string()
    )]
    #[case::pull_request_paths_sorted(
        Document::from_string(indoc! {"
            on:
                pull_request:
                    paths:
                        - 'tests/**'
                        - 'src/**'
                        - '*.json'
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            on:
              pull_request:
                paths:
                - '*.json'
                - 'src/**'
                - 'tests/**'
        "}.to_string()
    )]
    #[case::pull_request_target_branches_sorted(
        Document::from_string(indoc! {"
            on:
                pull_request_target:
                    branches:
                        - main
                        - develop
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            on:
              pull_request_target:
                branches:
                - develop
                - main
        "}.to_string()
    )]
    #[case::pull_request_target_branches_ignore_sorted(
        Document::from_string(indoc! {"
            on:
                pull_request_target:
                    branches-ignore:
                        - main
                        - develop
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            on:
              pull_request_target:
                branches-ignore:
                - develop
                - main
        "}.to_string()
    )]
    #[case::pull_request_target_paths_sorted(
        Document::from_string(indoc! {"
            on:
                pull_request_target:
                    paths:
                        - 'tests/**'
                        - 'src/**'
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            on:
              pull_request_target:
                paths:
                - 'src/**'
                - 'tests/**'
        "}.to_string()
    )]
    #[case::issues_types_sorted(
        Document::from_string(indoc! {"
            on:
                issues:
                    types:
                        - reopened
                        - deleted
                        - opened
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            on:
              issues:
                types:
                - deleted
                - opened
                - reopened
        "}.to_string()
    )]
    #[case::multiple_filters_sorted(
        Document::from_string(indoc! {"
            on:
                pull_request:
                    types:
                        - synchronize
                        - opened
                        - reopened
                    paths:
                        - 'tests/**'
                        - 'src/**'
                        - '*.json'
                    branches:
                        - main
                        - develop
                push:
                    tags:
                        - 'v2.*'
                        - 'v1.*'
                    branches:
                        - main
                        - develop
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            on:
              pull_request:
                types:
                - opened
                - reopened
                - synchronize
                paths:
                - '*.json'
                - 'src/**'
                - 'tests/**'
                branches:
                - develop
                - main
              push:
                tags:
                - 'v1.*'
                - 'v2.*'
                branches:
                - develop
                - main
        "}.to_string()
    )]
    #[case::already_sorted(
        Document::from_string(indoc! {"
            on:
                push:
                    branches:
                        - develop
                        - main
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            on:
              push:
                branches:
                - develop
                - main
        "}.to_string()
    )]
    fn test_filter_sorter(#[case] source_doc: Document, #[case] expected: String) {
        let result = FilterSorter::default()
            .process(source_doc)
            .expect("processing failed")
            .to_string();

        assert_eq!(result, expected);
    }
}
