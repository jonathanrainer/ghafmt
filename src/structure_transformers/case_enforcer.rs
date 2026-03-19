//! Transformer that enforces a consistent casing standard on user-controlled identifiers.
use std::{collections::HashMap, sync::LazyLock};

use fyaml::{Document, NodeRef, NodeStyle};
use regex::Regex;
use tracing::debug;

use crate::structure_transformers::StructureTransformer;

/// GHA identifier pattern: starts with letter/underscore, then alphanumeric/underscore/hyphen.
const ID_PATTERN: &str = r"[a-zA-Z_][a-zA-Z0-9_-]*";

/// Pre-compiled regex matching `needs` scalar or array-element paths within `/jobs`.
#[allow(clippy::unwrap_used)] // literal pattern — always a valid regex
static NEEDS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^/jobs/[^/]+/needs(/[0-9]+)?$").unwrap());
/// Reserved matrix keys, when we match matrix keys we have to ignore these because they're
/// termed special by GHA
const MATRIX_RESERVED: &[&str] = &["include", "exclude"];

/// A transformer that will intelligently apply the rename function to all user-controlled sites,
/// this allows it to enforce a particular casing standard throughout.
///
/// N.B. This will not work for outputs that are set inside of steps, as they are very challenging
/// to detect with 100% accuracy.
pub(crate) struct CaseEnforcer {
    /// A function that describes how to transform a single string into the correct casing.
    /// Usually taken from a library
    case_fn: fn(&str) -> String,
    /// A set of regexes that dictate where the keys that the enforcer operates on can be found
    key_regexes: Vec<Regex>,
    /// A set of regexes that dictates where any values the enforcer operator on can be found.
    /// This is particularly useful for step IDs as they are a scalar and not a key in the
    /// workflow YAML
    value_regexes: Vec<Regex>,
    /// A list of rules that indicate how to classify and replace values of each of these
    /// kinds.
    ref_kinds: Vec<RefKind>,
}

/// What type of rename are we working with?
enum Rename {
    /// A key in a YAML mapping
    Key(RenameEntry),
    /// A scalar value in a YAML mapping
    Value(RenameEntry),
}

/// A struct that describes a YPATH to a YAML key or value, the `old_name` it has and the new
/// name it should have
struct RenameEntry {
    /// A YPATH expression that describes where the key or value to be renamed is
    path: String,
    /// The name that key or value has in the original document
    old_name: String,
    /// The name it should have in the new document
    new_name: String,
}

impl Rename {
    /// Simple extractor function to get the rename entry from the enum
    fn entry(&self) -> &RenameEntry {
        match self {
            Rename::Key(e) | Rename::Value(e) => e,
        }
    }

    /// Check the depth of the path so we can operate on the deepest keys first
    fn depth(&self) -> usize {
        self.entry().path.matches('/').count()
    }
}

/// Defines how a category of identifier references is classified, matched, and rewritten.
struct RefRule {
    /// Matches document paths to determine if a rename belongs to this category.
    classify: Regex,
    /// Matches expression references (e.g. `needs.job_id`, `steps.step_id`).
    pattern: Regex,
    /// Template for the replacement string. Uses `{prefix}` for captured prefix and `{new}` for
    /// the renamed identifier.
    replacement: String,
}

/// Categories of identifier that can be renamed, each carrying its own matching rule.
enum RefKind {
    /// A job identifier (e.g. `jobs/my_job`).
    Job(RefRule),
    /// A step identifier (e.g. `steps/my_step/id`).
    Step(RefRule),
    /// A workflow input identifier.
    Input(RefRule),
    /// A workflow or job output identifier.
    Output(RefRule),
    /// A matrix dimension key.
    Matrix(RefRule),
    /// A workflow secret identifier.
    Secret(RefRule),
}

impl RefKind {
    /// Extract the underlying [`RefRule`] for this variant.
    fn rule(&self) -> &RefRule {
        match self {
            RefKind::Job(r)
            | RefKind::Step(r)
            | RefKind::Input(r)
            | RefKind::Output(r)
            | RefKind::Matrix(r)
            | RefKind::Secret(r) => r,
        }
    }
}

impl StructureTransformer for CaseEnforcer {
    fn process(&self, mut doc: Document) -> fyaml::Result<Document> {
        // Phase A: Collect renames
        let mut renames: Vec<Rename> = vec![];

        // Walk the keys of the YAML file to find all the key renames
        Self::walk_keys(&doc, &mut |path, key_str| {
            if self.key_regexes.iter().any(|r| r.is_match(path)) {
                // Make sure we don't try and rename the matrix.include/matrix.exclude keys
                // because they're special keys for GitHub Actions
                if MATRIX_RESERVED.contains(&key_str) {
                    return;
                }
                let new_name = (self.case_fn)(key_str);
                if key_str != new_name {
                    debug!(
                        "Found key rename at: '{}' from '{}' to '{}'",
                        path, key_str, new_name
                    );
                    renames.push(Rename::Key(RenameEntry {
                        path: path.to_string(),
                        old_name: key_str.to_string(),
                        new_name,
                    }));
                }
            }
        })?;

        Self::walk_scalars(&doc, &mut |path, val| {
            if self.value_regexes.iter().any(|r| r.is_match(path)) {
                let new_name = (self.case_fn)(val);
                if val != new_name {
                    debug!(
                        "Found value rename at: {} from '{}' to '{}'",
                        path, val, new_name
                    );
                    renames.push(Rename::Value(RenameEntry {
                        path: path.to_string(),
                        old_name: val.to_string(),
                        new_name,
                    }));
                }
            }
        })?;

        if renames.is_empty() {
            return Ok(doc);
        }

        // Phase B: Apply renames (deepest paths first so parent paths remain valid)
        renames.sort_by_key(|r| std::cmp::Reverse(r.depth()));

        for rename in &renames {
            let e = rename.entry();
            debug!("Renaming element at '{}' to '{}'", &e.path, &e.new_name);
            match rename {
                Rename::Key(_) => doc.edit().rename_key_at(&e.path, &e.new_name)?,
                Rename::Value(_) => doc.edit().set_yaml_at(&e.path, &e.new_name)?,
            }
        }

        // Phase C: Update references
        let ref_renames = self.classify_renames(&renames);

        // Find job renames for the bare-scalar `needs` rewriting
        let job_renames = self
            .ref_kinds
            .iter()
            .zip(ref_renames.iter())
            .find(|(kind, _)| matches!(kind, RefKind::Job(_)))
            .map(|(_, renames)| renames);

        let mut updates: Vec<(String, String)> = vec![];

        // Walk over the scalar values (as those are the only places you can have variable
        // substitutions) and calculate what the updates need to be.
        Self::walk_scalars(&doc, &mut |path, value| {
            if NEEDS_RE.is_match(path)
                && let Some(jobs) = job_renames
                && let Some(new_name) = jobs.get(value)
            {
                updates.push((path.to_string(), new_name.clone()));
                return;
            }

            if !value.contains("${{") {
                return;
            }

            let new_value = self.replace_refs(value, &ref_renames);

            if new_value != value {
                updates.push((path.to_string(), new_value));
            }
        })?;

        // Then mutate the document to perform the updates necessary
        for (path, new_value) in updates {
            // Check the existing style first: set_yaml_at with a bare multi-line string
            // succeeds (libfyaml folds newlines to spaces) rather than failing, so we
            // cannot rely on the error path to detect block scalars.  Read the style
            // up front and format the new value appropriately.
            let is_literal = doc
                .at_path(&path)
                .is_some_and(|n| n.style() == NodeStyle::Literal);
            if is_literal {
                let block = Self::format_as_yaml_literal_block(&new_value);
                doc.edit().set_yaml_at(&path, &block)?;
            } else if doc.edit().set_yaml_at(&path, &new_value).is_err() {
                doc.edit().set_scalar_at(&path, &new_value)?;
            }
        }

        Ok(doc)
    }

    fn name(&self) -> &'static str {
        "case-enforcer"
    }

    fn description(&self) -> &'static str {
        "Enforce consistent casing"
    }
}

impl CaseEnforcer {
    /// Build a `CaseEnforcer` that applies `case_fn` to all user-controlled identifiers.
    #[allow(clippy::unwrap_used)] // all patterns are string literals — always valid regexes
    pub(crate) fn new(case_fn: fn(&str) -> String) -> Self {
        let key_regexes = vec![
            Regex::new(r"^/jobs/[^/]+$").unwrap(),
            Regex::new(r"^/jobs/[^/]+/outputs/[^/]+$").unwrap(),
            Regex::new(r"^/jobs/[^/]+/strategy/matrix/[^/]+$").unwrap(),
            Regex::new(r"^/jobs/[^/]+/strategy/matrix/(?:include|exclude)/[0-9]+/[^/]+$").unwrap(),
            Regex::new(r"^/jobs/[^/]+/services/[^/]+$").unwrap(),
            Regex::new(r"^/on/workflow_dispatch/inputs/[^/]+$").unwrap(),
            Regex::new(r"^/on/workflow_call/inputs/[^/]+$").unwrap(),
            Regex::new(r"^/on/workflow_call/outputs/[^/]+$").unwrap(),
            Regex::new(r"^/on/workflow_call/secrets/[^/]+$").unwrap(),
        ];
        let value_regexes = vec![Regex::new(r"^/jobs/[^/]+/steps/[0-9]+/id$").unwrap()];

        let id = ID_PATTERN;
        let ref_kinds = vec![
            RefKind::Job(RefRule {
                classify: Regex::new(r"^/jobs/[^/]+$").unwrap(),
                pattern: Regex::new(&format!(r"(needs|jobs)\.({id})")).unwrap(),
                replacement: "{prefix}.{new}".into(),
            }),
            RefKind::Step(RefRule {
                classify: Regex::new(r"/steps/[0-9]+/id$").unwrap(),
                pattern: Regex::new(&format!(r"steps\.({id})")).unwrap(),
                replacement: "steps.{new}".into(),
            }),
            RefKind::Input(RefRule {
                classify: Regex::new(r"/inputs/[^/]+$").unwrap(),
                pattern: Regex::new(&format!(r"inputs\.({id})")).unwrap(),
                replacement: "inputs.{new}".into(),
            }),
            RefKind::Output(RefRule {
                // Only rewrite job-level outputs (needs.X.outputs / jobs.X.outputs),
                // NOT step-level outputs (steps.X.outputs) — step output names are
                // set by scripts at runtime, not by YAML keys.
                classify: Regex::new(r"/outputs/[^/]+$").unwrap(),
                pattern: Regex::new(&format!(r"((?:needs|jobs)\.[^.]+)\.outputs\.({id})")).unwrap(),
                replacement: "{prefix}.outputs.{new}".into(),
            }),
            RefKind::Matrix(RefRule {
                classify: Regex::new(r"/strategy/matrix/").unwrap(),
                pattern: Regex::new(&format!(r"matrix\.({id})")).unwrap(),
                replacement: "matrix.{new}".into(),
            }),
            RefKind::Secret(RefRule {
                classify: Regex::new(r"/secrets/[^/]+$").unwrap(),
                pattern: Regex::new(&format!(r"secrets\.({id})")).unwrap(),
                replacement: "secrets.{new}".into(),
            }),
        ];

        Self {
            case_fn,
            key_regexes,
            value_regexes,
            ref_kinds,
        }
    }

    /// Walk over all the keys of all the mappings in the YAML document and apply the
    /// visitor function to all of them.
    fn walk_keys<F>(doc: &Document, visitor: &mut F) -> fyaml::Result<()>
    where
        F: FnMut(&str, &str),
    {
        if let Some(root) = doc.root() {
            Self::walk_keys_rec(root, "", visitor)?;
        }
        Ok(())
    }

    /// Helper function to allow `walk_keys` to do some extra checking while still being recursive
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    fn walk_keys_rec<F>(node: NodeRef, path: &str, visitor: &mut F) -> fyaml::Result<()>
    where
        F: FnMut(&str, &str),
    {
        if node.is_mapping() {
            for (key, value) in node.map_iter() {
                let key_str = key.scalar_str()?;
                let child_path = format!("{path}/{key_str}");
                visitor(&child_path, key_str);
                Self::walk_keys_rec(value, &child_path, visitor)?;
            }
        } else if node.is_sequence() {
            for i in 0..node.seq_len()? {
                if let Some(item) = node.seq_get(i as i32) {
                    Self::walk_keys_rec(item, &format!("{path}/{i}"), visitor)?;
                }
            }
        }
        Ok(())
    }

    /// Walk over all the scalar values in the YAML file and apply the visitor function to each one
    fn walk_scalars<F>(doc: &Document, visitor: &mut F) -> fyaml::Result<()>
    where
        F: FnMut(&str, &str),
    {
        if let Some(root) = doc.root() {
            Self::walk_scalars_rec(root, "", visitor)?;
        }
        Ok(())
    }

    /// Recursive implementation of [`walk_scalars`](Self::walk_scalars).
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    fn walk_scalars_rec<F>(node: NodeRef, path: &str, visitor: &mut F) -> fyaml::Result<()>
    where
        F: FnMut(&str, &str),
    {
        if node.is_mapping() {
            for (key, value) in node.map_iter() {
                let child_path = format!("{}/{}", path, key.scalar_str()?);
                Self::walk_scalars_rec(value, &child_path, visitor)?;
            }
        } else if node.is_sequence() {
            for i in 0..node.seq_len()? {
                if let Some(item) = node.seq_get(i as i32) {
                    Self::walk_scalars_rec(item, &format!("{path}/{i}"), visitor)?;
                }
            }
        } else if node.is_scalar() {
            visitor(path, node.scalar_str()?);
        }
        Ok(())
    }

    /// Convert a set of renames in a list of renames we need to make
    fn classify_renames(&self, renames: &[Rename]) -> Vec<HashMap<String, String>> {
        let mut ref_renames: Vec<HashMap<String, String>> =
            vec![HashMap::new(); self.ref_kinds.len()];
        for rename in renames {
            let e = rename.entry();
            for (i, ref_kind) in self.ref_kinds.iter().enumerate() {
                if ref_kind.rule().classify.is_match(&e.path) {
                    debug!(
                        "Planning rename from '{}' to '{}' at '{}'",
                        e.old_name.clone(),
                        e.new_name.clone(),
                        e.path
                    );
                    ref_renames[i].insert(e.old_name.clone(), e.new_name.clone());
                    break;
                }
            }
        }
        ref_renames
    }

    /// Format `content` as a YAML literal block scalar (`|`).
    ///
    /// Used to preserve `|` block-scalar style when updating a multi-line scalar
    /// whose content contains `${{ }}` expressions: `set_yaml_at` rejects bare
    /// newlines as inline YAML, but accepts a properly-formed `|\n  …` snippet.
    fn format_as_yaml_literal_block(content: &str) -> String {
        let mut result = String::from("|\n");
        for line in content.lines() {
            result.push_str("  ");
            result.push_str(line);
            result.push('\n');
        }
        result
    }

    /// Actually apply a set of renames to a particular string value
    fn replace_refs(&self, value: &str, ref_renames: &[HashMap<String, String>]) -> String {
        let mut result = value.to_string();

        for (ref_kind, renames) in self.ref_kinds.iter().zip(ref_renames.iter()) {
            if renames.is_empty() {
                continue;
            }

            let rule = ref_kind.rule();
            let template = &rule.replacement;

            result = rule
                .pattern
                .replace_all(&result, |caps: &regex::Captures| {
                    // The identifier to look up is always the last capture group.
                    let id = &caps[caps.len() - 1];
                    match renames.get(id) {
                        Some(new) => {
                            let mut out = template.replace("{new}", new);
                            // If there's a prefix capture group (group count > 2, i.e. prefix + id),
                            // substitute it.
                            if caps.len() > 2 {
                                out = out.replace("{prefix}", &caps[1]);
                            }
                            out
                        }
                        None => caps[0].to_string(),
                    }
                })
                .into_owned();
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use rstest::rstest;
    use similar_asserts::assert_eq;

    use super::*;

    fn snake_case_enforcer() -> CaseEnforcer {
        CaseEnforcer::new(heck::ToSnakeCase::to_snake_case)
    }

    fn rename(path: &str, old: &str, new: &str) -> Rename {
        Rename::Key(RenameEntry {
            path: path.to_string(),
            old_name: old.to_string(),
            new_name: new.to_string(),
        })
    }

    #[rstest]
    #[case::job_via_needs(
        "${{ needs.myJob.outputs.foo }}",
        vec![rename("/jobs/myJob", "myJob", "my_job")],
        "${{ needs.my_job.outputs.foo }}"
    )]
    #[case::job_via_jobs(
        "${{ jobs.myJob.result }}",
        vec![rename("/jobs/myJob", "myJob", "my_job")],
        "${{ jobs.my_job.result }}"
    )]
    #[case::step_ref(
        "${{ steps.myStep.outputs.foo }}",
        vec![rename("/jobs/build/steps/0/id", "myStep", "my_step")],
        "${{ steps.my_step.outputs.foo }}"
    )]
    #[case::input_ref(
        "${{ inputs.myInput }}",
        vec![rename("/on/workflow_call/inputs/myInput", "myInput", "my_input")],
        "${{ inputs.my_input }}"
    )]
    #[case::matrix_ref(
        "${{ matrix.myVar }}",
        vec![rename("/jobs/build/strategy/matrix/myVar", "myVar", "my_var")],
        "${{ matrix.my_var }}"
    )]
    #[case::secret_ref(
        "${{ secrets.mySecret }}",
        vec![rename("/on/workflow_call/secrets/mySecret", "mySecret", "my_secret")],
        "${{ secrets.my_secret }}"
    )]
    #[case::job_output_ref(
        "${{ needs.myJob.outputs.myOutput }}",
        vec![
            rename("/jobs/myJob", "myJob", "my_job"),
            rename("/jobs/myJob/outputs/myOutput", "myOutput", "my_output")
        ],
        "${{ needs.my_job.outputs.my_output }}"
    )]
    #[case::step_output_not_rewritten(
        "${{ steps.myStep.outputs.myOutput }}",
        vec![
            rename("/jobs/build/steps/0/id", "myStep", "my_step"),
            rename("/jobs/build/outputs/myOutput", "myOutput", "my_output")
        ],
        "${{ steps.my_step.outputs.myOutput }}"
    )]
    #[case::no_matching_rename(
        "${{ needs.unknownJob.outputs.foo }}",
        vec![rename("/jobs/myJob", "myJob", "my_job")],
        "${{ needs.unknownJob.outputs.foo }}"
    )]
    #[case::multiple_refs_in_one_expression(
        "${{ needs.myJob.result == 'success' && steps.myStep.outputs.foo }}",
        vec![
            rename("/jobs/myJob", "myJob", "my_job"),
            rename("/jobs/build/steps/0/id", "myStep", "my_step")
        ],
        "${{ needs.my_job.result == 'success' && steps.my_step.outputs.foo }}"
    )]
    fn test_replace_refs(
        #[case] input: &str,
        #[case] renames: Vec<Rename>,
        #[case] expected: &str,
    ) {
        let enforcer = snake_case_enforcer();
        let ref_renames = enforcer.classify_renames(&renames);
        let result = enforcer.replace_refs(input, &ref_renames);
        assert_eq!(result, expected);
    }

    #[rstest]
    #[case::renames_job_ids(
        indoc! {"
            on: push
            jobs:
              myJob:
                runs-on: ubuntu-latest
        "},
        indoc! {"
            on: push
            jobs:
              my_job:
                runs-on: ubuntu-latest
        "}
    )]
    #[case::renames_step_ids(
        indoc! {"
            on: push
            jobs:
              build:
                runs-on: ubuntu-latest
                steps:
                - id: myStep
                  run: echo hi
        "},
        indoc! {"
            on: push
            jobs:
              build:
                runs-on: ubuntu-latest
                steps:
                - id: my_step
                  run: echo hi
        "}
    )]
    #[case::updates_needs_references(
        indoc! {"
            on: push
            jobs:
              buildJob:
                runs-on: ubuntu-latest
              deployJob:
                runs-on: ubuntu-latest
                needs: buildJob
        "},
        indoc! {"
            on: push
            jobs:
              build_job:
                runs-on: ubuntu-latest
              deploy_job:
                runs-on: ubuntu-latest
                needs: build_job
        "}
    )]
    #[case::preserves_matrix_reserved_keys(
        indoc! {"
            on: push
            jobs:
              build:
                runs-on: ubuntu-latest
                strategy:
                  matrix:
                    include:
                    - myVar: value
        "},
        indoc! {"
            on: push
            jobs:
              build:
                runs-on: ubuntu-latest
                strategy:
                  matrix:
                    include:
                    - my_var: value
        "}
    )]
    #[case::already_snake_case_is_noop(
        indoc! {"
            on: push
            jobs:
              my_job:
                runs-on: ubuntu-latest
        "},
        indoc! {"
            on: push
            jobs:
              my_job:
                runs-on: ubuntu-latest
        "}
    )]
    #[case::updates_expression_refs(
        indoc! {"
            on: push
            jobs:
              myJob:
                runs-on: ubuntu-latest
                outputs:
                  myOutput: value
              other:
                runs-on: ubuntu-latest
                needs: myJob
                env:
                  FOO: ${{ needs.myJob.outputs.myOutput }}
        "},
        indoc! {"
            on: push
            jobs:
              my_job:
                runs-on: ubuntu-latest
                outputs:
                  my_output: value
              other:
                runs-on: ubuntu-latest
                needs: my_job
                env:
                  FOO: ${{ needs.my_job.outputs.my_output }}
        "}
    )]
    fn test_process(#[case] input: &str, #[case] expected: &str) {
        let enforcer = snake_case_enforcer();
        let doc = Document::parse_str(input).expect("parse failed");
        let result = enforcer.process(doc).expect("process failed").to_string();
        assert_eq!(expected, &result);
    }
}
