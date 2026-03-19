//! Sorts keys under `container` and `services` within a job into idiomatic order.
use fyaml::Document;

use crate::structure_transformers::{StructureTransformer, for_each_mapping_child};

/// Canonical key order within a `container` or `services` entry.
const CONTAINER_ORDERING: [&str; 6] =
    ["image", "credentials", "env", "ports", "volumes", "options"];

/// Sorts keys under `container` and `services` within a job into idiomatic order.
pub(crate) struct ContainerSorter {
    /// Pre-computed key ordering to avoid allocating on every call.
    key_ordering: Vec<String>,
}

impl Default for ContainerSorter {
    fn default() -> Self {
        Self {
            key_ordering: CONTAINER_ORDERING.map(String::from).to_vec(),
        }
    }
}

impl StructureTransformer for ContainerSorter {
    fn process(&self, mut doc: Document) -> fyaml::Result<Document> {
        // For each job, sort the mapping under the key 'container', then also sort each of
        // the services under the key 'services'
        //
        // Both of those keys expose either a Container, or a list of Containers
        doc = for_each_mapping_child(doc, "jobs", |doc, job_path| {
            let doc = self.sort_mapping_at_path(
                doc,
                &format!("{job_path}/container"),
                &self.key_ordering,
            )?;
            for_each_mapping_child(doc, &format!("{job_path}/services"), |doc, service_path| {
                self.sort_mapping_at_path(doc, service_path, &self.key_ordering)
            })
        })?;
        Ok(doc)
    }

    fn name(&self) -> &'static str {
        "container-sorter"
    }

    fn description(&self) -> &'static str {
        "Sort keys under 'container' and 'services' within a job"
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
    #[case::no_container_key(
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
    #[case::container_keys_reordered(
        Document::from_string(indoc! {"
            jobs:
                build:
                    container:
                        options: --cpus 2
                        volumes:
                            - /tmp:/tmp
                        ports:
                            - 8080:80
                        env:
                            CI: true
                        credentials:
                            username: user
                            password: pass
                        image: node:20
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              build:
                container:
                  image: node:20
                  credentials:
                    username: user
                    password: pass
                  env:
                    CI: true
                  ports:
                  - 8080:80
                  volumes:
                  - /tmp:/tmp
                  options: --cpus 2
        "}.to_string()
    )]
    #[case::service_keys_reordered(
        Document::from_string(indoc! {"
            jobs:
                test:
                    services:
                        postgres:
                            options: --health-cmd pg_isready
                            ports:
                                - 5432:5432
                            env:
                                POSTGRES_DB: test
                            image: postgres:15
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              test:
                services:
                  postgres:
                    image: postgres:15
                    env:
                      POSTGRES_DB: test
                    ports:
                    - 5432:5432
                    options: --health-cmd pg_isready
        "}.to_string()
    )]
    #[case::multiple_services(
        Document::from_string(indoc! {"
            jobs:
                test:
                    services:
                        redis:
                            ports:
                                - 6379:6379
                            image: redis:7
                        postgres:
                            ports:
                                - 5432:5432
                            image: postgres:15
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              test:
                services:
                  redis:
                    image: redis:7
                    ports:
                    - 6379:6379
                  postgres:
                    image: postgres:15
                    ports:
                    - 5432:5432
        "}.to_string()
    )]
    #[case::container_and_services_together(
        Document::from_string(indoc! {"
            jobs:
                test:
                    container:
                        env:
                            NODE_ENV: test
                        image: node:20
                    services:
                        redis:
                            ports:
                                - 6379:6379
                            image: redis:7
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              test:
                container:
                  image: node:20
                  env:
                    NODE_ENV: test
                services:
                  redis:
                    image: redis:7
                    ports:
                    - 6379:6379
        "}.to_string()
    )]
    #[case::multiple_jobs_with_containers(
        Document::from_string(indoc! {"
            jobs:
                build:
                    container:
                        env:
                            CI: true
                        image: node:20
                deploy:
                    container:
                        options: --cpus 1
                        image: alpine:3
        "}.to_string()).expect("test input is valid YAML"),
        indoc! {"
            jobs:
              build:
                container:
                  image: node:20
                  env:
                    CI: true
              deploy:
                container:
                  image: alpine:3
                  options: --cpus 1
        "}.to_string()
    )]
    fn test_container_sorter(#[case] source_doc: Document, #[case] expected: String) {
        let result = ContainerSorter::default()
            .process(source_doc)
            .expect("processing failed")
            .to_string();

        assert_eq!(result, expected);
    }
}
