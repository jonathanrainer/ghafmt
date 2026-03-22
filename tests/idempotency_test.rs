use std::{fs::read_to_string, path::PathBuf};

use fyaml::Document;
use ghafmt::Ghafmt;
use patharg::InputArg;
use rstest::rstest;
use similar_asserts::assert_eq;

#[rstest]
fn test_idempotency(#[files("tests/fixtures/clean/**/*.yaml")] path: PathBuf) {
    let original = read_to_string(&path).unwrap_or_else(|_| panic!("{} not found", path.display()));

    let mut formatter = Ghafmt::default();
    let doc = Document::from_string(original.clone()).expect("Should be valid YAML");

    let (formatted, _) = formatter
        .format_gha_document(doc, &InputArg::Path(path.clone()))
        .expect("Could not format workflow");

    assert_eq!(original, formatted, "Formatting is not idempotent");
}
