//! Sorts the keys under the job-level `environment` mapping into idiomatic order.
use fyaml::Document;

use crate::structure_transformers::{StructureTransformer, for_each_mapping_child};

/// Canonical key order within a job-level `environment` mapping.
const ENVIRONMENT_ORDERING: [&str; 2] = ["name", "url"];

/// Sorts the keys under the job-level `environment` mapping into idiomatic order.
pub(crate) struct EnvironmentSorter {
    /// Pre-computed key ordering to avoid allocating on every call.
    key_ordering: Vec<String>,
}

impl Default for EnvironmentSorter {
    fn default() -> Self {
        Self {
            key_ordering: ENVIRONMENT_ORDERING.map(String::from).to_vec(),
        }
    }
}

impl StructureTransformer for EnvironmentSorter {
    fn process(&self, mut doc: Document) -> fyaml::Result<Document> {
        doc = for_each_mapping_child(doc, "jobs", |doc, job_path| {
            self.sort_mapping_at_path(doc, &format!("{job_path}/environment"), &self.key_ordering)
        })?;
        Ok(doc)
    }

    fn name(&self) -> &'static str {
        "environment-sorter"
    }

    fn description(&self) -> &'static str {
        "Sort the entries under the 'environment' (as distinct from 'env') key in each job into idiomatic order"
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
    #[case::no_environment(
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
    #[case::environment_keys_reordered(
        Document::from_string(indoc! {"
            jobs:
                deploy:
                    environment:
                        url: https://example.com
                        name: production
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              deploy:
                environment:
                  name: production
                  url: https://example.com
        "}.to_string()
    )]
    #[case::environment_already_ordered(
        Document::from_string(indoc! {"
            jobs:
                deploy:
                    environment:
                        name: staging
                        url: https://staging.example.com
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              deploy:
                environment:
                  name: staging
                  url: https://staging.example.com
        "}.to_string()
    )]
    #[case::multiple_jobs(
        Document::from_string(indoc! {"
            jobs:
                staging:
                    environment:
                        url: https://staging.example.com
                        name: staging
                production:
                    environment:
                        url: https://example.com
                        name: production
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              staging:
                environment:
                  name: staging
                  url: https://staging.example.com
              production:
                environment:
                  name: production
                  url: https://example.com
        "}.to_string()
    )]
    #[case::scalar_environment_unchanged(
        Document::from_string(indoc! {"
            jobs:
                deploy:
                    environment: production
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              deploy:
                environment: production
        "}.to_string()
    )]
    fn test_environment_sorter(#[case] source_doc: Document, #[case] expected: String) {
        let result = EnvironmentSorter::default()
            .process(source_doc)
            .expect("processing failed")
            .to_string();

        assert_eq!(result, expected);
    }
}
