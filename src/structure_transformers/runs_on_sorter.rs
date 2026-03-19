//! Sorts all `runs-on` entries, appropriate to their underlying YAML type.
use fyaml::{Document, NodeType};

use crate::structure_transformers::{StructureTransformer, for_each_mapping_child};

/// Canonical key order within a `runs-on` mapping (group-runner form).
const RUNS_ON_KEYS: [&str; 2] = ["group", "labels"];

/// Sorts all `runs-on` entries, appropriate to their underlying YAML type.
pub(crate) struct RunsOnSorter {
    /// Pre-computed key ordering to avoid allocating on every call.
    key_ordering: Vec<String>,
}

impl Default for RunsOnSorter {
    fn default() -> Self {
        Self {
            key_ordering: RUNS_ON_KEYS.map(String::from).to_vec(),
        }
    }
}

impl StructureTransformer for RunsOnSorter {
    fn process(&self, mut doc: Document) -> fyaml::Result<Document> {
        doc = for_each_mapping_child(doc, "jobs", |doc, job_path| {
            let job_runs_on_path = format!("{job_path}/runs-on");
            // Apply the correct type of sorting, ignoring Nones and any other NodeTypes
            match doc.at_path(&job_runs_on_path).map(|n| n.kind()) {
                Some(NodeType::Sequence) => {
                    self.sort_seq_at_path_alphabetically(doc, &job_runs_on_path)
                }
                Some(NodeType::Mapping) => {
                    self.sort_mapping_at_path(doc, &job_runs_on_path, &self.key_ordering)
                }
                _ => Ok(doc),
            }
        })?;

        Ok(doc)
    }

    fn name(&self) -> &'static str {
        "runs-on-sorter"
    }

    fn description(&self) -> &'static str {
        "Sort all 'runs-on' entries, appropriate to their underlying type"
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
    #[case::scalar_runs_on_unchanged(
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
    #[case::sequence_sorted(
        Document::from_string(indoc! {"
            jobs:
                build:
                    runs-on:
                        - self-hosted
                        - linux
                        - x64
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              build:
                runs-on:
                - linux
                - self-hosted
                - x64
        "}.to_string()
    )]
    #[case::mapping_sorted(
        Document::from_string(indoc! {"
            jobs:
                build:
                    runs-on:
                        labels:
                            - linux
                            - x64
                        group: my-runner-group
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              build:
                runs-on:
                  group: my-runner-group
                  labels:
                  - linux
                  - x64
        "}.to_string()
    )]
    #[case::multiple_jobs_different_types(
        Document::from_string(indoc! {"
            jobs:
                scalar_job:
                    runs-on: ubuntu-latest
                sequence_job:
                    runs-on:
                        - self-hosted
                        - linux
                group_job:
                    runs-on:
                        labels:
                            - x64
                        group: runners
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              scalar_job:
                runs-on: ubuntu-latest
              sequence_job:
                runs-on:
                - linux
                - self-hosted
              group_job:
                runs-on:
                  group: runners
                  labels:
                  - x64
        "}.to_string()
    )]
    #[case::sequence_already_sorted(
        Document::from_string(indoc! {"
            jobs:
                build:
                    runs-on:
                        - linux
                        - self-hosted
                        - x64
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              build:
                runs-on:
                - linux
                - self-hosted
                - x64
        "}.to_string()
    )]
    fn test_runs_on_sorter(#[case] source_doc: Document, #[case] expected: String) {
        let result = RunsOnSorter::default()
            .process(source_doc)
            .expect("processing failed")
            .to_string();

        assert_eq!(result, expected);
    }
}
