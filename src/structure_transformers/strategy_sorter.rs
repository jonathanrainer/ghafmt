//! Sorts keys under `strategy` within a job, including matrix dimension sorting.
use std::cmp::Ordering;

use fyaml::{Document, NodeType};

use crate::structure_transformers::{
    StructureTransformer, for_each_mapping_child, for_each_seq_element,
};

/// Canonical key order within a `strategy` mapping.
const STRATEGY_ORDER: [&str; 3] = ["fail-fast", "max-parallel", "matrix"];

/// Sorts keys under `strategy` within a job, including matrix dimension sorting.
pub(crate) struct StrategySorter {
    /// Pre-computed key ordering to avoid allocating on every call.
    strategy_ordering: Vec<String>,
}

impl Default for StrategySorter {
    fn default() -> Self {
        Self {
            strategy_ordering: STRATEGY_ORDER.map(String::from).to_vec(),
        }
    }
}

impl StructureTransformer for StrategySorter {
    #[allow(clippy::match_same_arms, clippy::unwrap_used)]
    // scalar_str() returns Err if the node is not a scalar or contains non-UTF-8 bytes;
    // both are invariants that hold for any well-formed GHA workflow.
    fn process(&self, mut doc: Document) -> fyaml::Result<Document> {
        doc = for_each_mapping_child(doc, "jobs", |mut doc, job_path| {
            let strategy_path = format!("{job_path}/strategy");
            if doc.at_path(&strategy_path).is_none() {
                return Ok(doc);
            }

            let matrix_path = format!("{strategy_path}/matrix");

            doc = self.sort_mapping_at_path(doc, &strategy_path, &self.strategy_ordering)?;

            // Sort matrix keys: custom dimension keys alphabetically, include/exclude last
            doc.edit().sort_mapping_at(&matrix_path, |k1, _, k2, _| {
                let key1 = k1.scalar_str().unwrap();
                let key2 = k2.scalar_str().unwrap();
                match (key1, key2) {
                    ("include", "exclude") => Ordering::Less,
                    ("exclude", "include") => Ordering::Greater,
                    (_, "include" | "exclude") => Ordering::Less,
                    ("include" | "exclude", _) => Ordering::Greater,
                    (_, _) => key1.cmp(key2),
                }
            })?;

            // Collect dimension keys (exclude include/exclude) for ordering entries later
            let mut dimension_keys: Vec<String> = doc
                .at_path(&matrix_path)
                .map(|n| {
                    n.map_iter()
                        .map(|(k, _)| k.scalar_str().unwrap().to_string())
                        .filter(|k| k != "include" && k != "exclude")
                        .collect()
                })
                .unwrap_or_default();
            dimension_keys.sort();

            // Sort sequences within each dimension key
            for key in &dimension_keys {
                let key_path = format!("{matrix_path}/{key}");
                match doc
                    .at_path(&key_path)
                    .and_then(|n| n.seq_get(0))
                    .map(|n| n.kind())
                {
                    None => {}
                    Some(NodeType::Scalar) => {
                        doc = self.sort_seq_at_path_alphabetically(doc, &key_path)?;
                    }
                    Some(NodeType::Mapping) => {
                        doc = for_each_seq_element(doc, &key_path, |doc, entry_path| {
                            self.sort_path_to_mapping_alphabetically(doc, entry_path)
                        })?;
                    }
                    Some(NodeType::Sequence) => unreachable!(
                        "Sequences of sequences are not supported for matrix strategies"
                    ),
                }
            }

            // Sort keys within include/exclude entries: dimension keys first, then extras
            for special_key in ["include", "exclude"] {
                let special_path = format!("{matrix_path}/{special_key}");
                doc = for_each_seq_element(doc, &special_path, |doc, entry_path| {
                    let Some(node) = doc.at_path(entry_path) else {
                        return Ok(doc);
                    };
                    let mut extra_keys: Vec<String> = node
                        .map_iter()
                        .map(|(k, _)| k.scalar_str().unwrap().to_owned())
                        .filter(|k| !dimension_keys.contains(k))
                        .collect();
                    extra_keys.sort();
                    let key_order: Vec<String> =
                        dimension_keys.iter().cloned().chain(extra_keys).collect();
                    self.sort_mapping_at_path(doc, entry_path, &key_order)
                })?;
            }

            Ok(doc)
        })?;

        Ok(doc)
    }

    fn name(&self) -> &'static str {
        "strategy-sorter"
    }

    fn description(&self) -> &'static str {
        "Sort keys under 'strategy' within a job"
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
    #[case::no_strategy(
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
    #[case::strategy_keys_sorted(
        Document::from_string(indoc! {"
            jobs:
                test:
                    strategy:
                        max-parallel: 4
                        matrix:
                            os:
                                - ubuntu-latest
                        fail-fast: false
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              test:
                strategy:
                  fail-fast: false
                  max-parallel: 4
                  matrix:
                    os:
                    - ubuntu-latest
        "}.to_string()
    )]
    #[case::matrix_custom_keys_sorted(
        Document::from_string(indoc! {"
            jobs:
                test:
                    strategy:
                        matrix:
                            os:
                                - ubuntu-latest
                            node:
                                - '20'
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              test:
                strategy:
                  matrix:
                    node:
                    - '20'
                    os:
                    - ubuntu-latest
        "}.to_string()
    )]
    #[case::matrix_scalar_values_sorted(
        Document::from_string(indoc! {"
            jobs:
                test:
                    strategy:
                        matrix:
                            node:
                                - '22'
                                - '18'
                                - '20'
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              test:
                strategy:
                  matrix:
                    node:
                    - '18'
                    - '20'
                    - '22'
        "}.to_string()
    )]
    #[case::include_exclude_last(
        Document::from_string(indoc! {"
            jobs:
                test:
                    strategy:
                        matrix:
                            include:
                                - os: ubuntu-latest
                                  node: '22'
                            os:
                                - ubuntu-latest
                            exclude:
                                - os: ubuntu-latest
                                  node: '18'
                            node:
                                - '18'
                                - '20'
                                - '22'
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              test:
                strategy:
                  matrix:
                    node:
                    - '18'
                    - '20'
                    - '22'
                    os:
                    - ubuntu-latest
                    include:
                    - node: '22'
                      os: ubuntu-latest
                    exclude:
                    - node: '18'
                      os: ubuntu-latest
        "}.to_string()
    )]
    #[case::expression_valued_include_exclude(
        Document::from_string(indoc! {"
            jobs:
                test:
                    strategy:
                        matrix:
                            include: ${{ fromJSON(inputs.platforms) }}
                            exclude: ${{ fromJSON(inputs.excludes) }}
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              test:
                strategy:
                  matrix:
                    include: ${{ fromJSON(inputs.platforms) }}
                    exclude: ${{ fromJSON(inputs.excludes) }}
        "}.to_string()
    )]
    #[case::expression_valued_dimension_key(
        Document::from_string(indoc! {"
            jobs:
                test:
                    strategy:
                        matrix:
                            platforms: ${{ fromJSON(inputs.platforms) }}
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              test:
                strategy:
                  matrix:
                    platforms: ${{ fromJSON(inputs.platforms) }}
        "}.to_string()
    )]
    #[case::already_sorted(
        Document::from_string(indoc! {"
            jobs:
                test:
                    strategy:
                        fail-fast: false
                        matrix:
                            node:
                                - '18'
                                - '20'
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              test:
                strategy:
                  fail-fast: false
                  matrix:
                    node:
                    - '18'
                    - '20'
        "}.to_string()
    )]
    fn test_strategy_sorter(#[case] source_doc: Document, #[case] expected: String) {
        let result = StrategySorter::default()
            .process(source_doc)
            .expect("processing failed")
            .to_string();

        assert_eq!(result, expected);
    }
}
