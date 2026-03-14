use std::path::PathBuf;

use ghafmt::{Error, Ghafmt};
use rstest::rstest;

#[rstest]
fn test_parse_error(#[files("tests/fixtures/broken/*.yaml")] path: PathBuf) {
    let mut formatter = Ghafmt::new();
    let result = formatter.format_gha_workflow(&path);
    assert!(
        matches!(result, Err(Error::ParseYaml { .. })),
        "expected ParseYaml error for {}, got: {result:?}",
        path.display()
    );
}

#[test]
fn test_read_file_error() {
    let mut formatter = Ghafmt::new();
    let result = formatter
        .format_gha_workflow(PathBuf::from("tests/fixtures/broken/does_not_exist.yaml").as_path());
    assert!(
        matches!(result, Err(Error::ReadFile { .. })),
        "expected ReadFile error, got: {result:?}"
    );
}
