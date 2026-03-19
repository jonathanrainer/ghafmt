//! Sorts the keys of each job within a GitHub Actions workflow into idiomatic order.
use fyaml::Document;

use crate::structure_transformers::{StructureTransformer, for_each_mapping_child};

/// Canonical key order for the top-level keys of each job.
const JOB_LEVEL_KEY_ORDERING: [&str; 20] = [
    "name",
    "needs",
    "if",
    "runs-on",
    "snapshot",
    "environment",
    "permissions",
    "concurrency",
    "container",
    "services",
    "strategy",
    "timeout-minutes",
    "continue-on-error",
    "env",
    "defaults",
    "outputs",
    "uses",
    "with",
    "secrets",
    "steps",
];

/// Sorts the keys of each job into the canonical GHA ordering.
pub(crate) struct JobSorter {
    /// Pre-computed key ordering to avoid allocating on every call.
    key_ordering: Vec<String>,
}

impl Default for JobSorter {
    fn default() -> Self {
        Self {
            key_ordering: JOB_LEVEL_KEY_ORDERING.map(String::from).to_vec(),
        }
    }
}

impl StructureTransformer for JobSorter {
    fn process(&self, mut doc: Document) -> fyaml::Result<Document> {
        doc = for_each_mapping_child(doc, "/jobs", |doc, job_path| {
            let doc = self.sort_mapping_at_path(doc, job_path, &self.key_ordering)?;
            let doc =
                self.sort_path_to_mapping_alphabetically(doc, &format!("{job_path}/outputs"))?;
            self.sort_path_to_mapping_alphabetically(doc, &format!("{job_path}/secrets"))
        })?;
        Ok(doc)
    }

    fn name(&self) -> &'static str {
        "job-sorter"
    }

    fn description(&self) -> &'static str {
        "Sort job keys into idiomatic order"
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
    #[case::no_jobs(
        Document::from_string(indoc! {"
            on: push
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            on: push
        "}.to_string()
    )]
    #[case::job_keys_reordered(
        Document::from_string(indoc! {"
            jobs:
                build:
                    steps:
                        - run: echo hi
                    runs-on: ubuntu-latest
                    env:
                        CI: true
                    name: Build
                    if: success()
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              build:
                name: Build
                if: success()
                runs-on: ubuntu-latest
                env:
                  CI: true
                steps:
                - run: echo hi
        "}.to_string()
    )]
    #[case::outputs_sorted_alphabetically(
        Document::from_string(indoc! {"
            jobs:
                build:
                    outputs:
                        z_output: z
                        a_output: a
                        m_output: m
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              build:
                outputs:
                  a_output: a
                  m_output: m
                  z_output: z
        "}.to_string()
    )]
    #[case::secrets_sorted_alphabetically(
        Document::from_string(indoc! {"
            jobs:
                call:
                    secrets:
                        z_secret: val
                        a_secret: val
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              call:
                secrets:
                  a_secret: val
                  z_secret: val
        "}.to_string()
    )]
    #[case::multiple_jobs_all_sorted(
        Document::from_string(indoc! {"
            jobs:
                build:
                    steps:
                        - run: echo build
                    runs-on: ubuntu-latest
                    name: Build
                deploy:
                    env:
                        STAGE: prod
                    runs-on: ubuntu-latest
                    needs: build
                    name: Deploy
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              build:
                name: Build
                runs-on: ubuntu-latest
                steps:
                - run: echo build
              deploy:
                name: Deploy
                needs: build
                runs-on: ubuntu-latest
                env:
                  STAGE: prod
        "}.to_string()
    )]
    #[case::already_ordered(
        Document::from_string(indoc! {"
            jobs:
                build:
                    name: Build
                    runs-on: ubuntu-latest
                    env:
                        CI: true
                    steps:
                        - run: echo hi
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              build:
                name: Build
                runs-on: ubuntu-latest
                env:
                  CI: true
                steps:
                - run: echo hi
        "}.to_string()
    )]
    fn test_job_sorter(#[case] source_doc: Document, #[case] expected: String) {
        let result = JobSorter::default()
            .process(source_doc)
            .expect("processing failed")
            .to_string();

        assert_eq!(result, expected);
    }
}
