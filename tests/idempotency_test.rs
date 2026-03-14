use std::{fs::read_to_string, path::PathBuf};

use ghafmt::Ghafmt;
use rstest::rstest;
use similar_asserts::assert_eq;

#[rstest]
fn test_idempotency(#[files("tests/fixtures/clean/*.yaml")] path: PathBuf) {
    let original = read_to_string(&path).unwrap_or_else(|_| panic!("{} not found", path.display()));

    let mut formatter = Ghafmt::new();

    let (formatted, _) = formatter
        .format_gha_workflow(&path)
        .expect("Could not format workflow");

    assert_eq!(original, formatted, "Formatting is not idempotent");
}

#[rstest]
#[ignore = "pending fixtures are works in progress"]
fn test_idempotency_pending(#[files("tests/fixtures/pending/clean/*.yaml")] path: PathBuf) {
    let original = read_to_string(&path).unwrap_or_else(|_| panic!("{} not found", path.display()));

    let mut formatter = Ghafmt::new();

    let (formatted, _) = formatter
        .format_gha_workflow(&path)
        .expect("Could not format workflow");

    assert_eq!(original, formatted, "Formatting is not idempotent");
}
