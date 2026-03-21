use std::{fs, path::PathBuf};

use fyaml::Document;
use ghafmt::{Error, Ghafmt};
use rstest::rstest;

#[rstest]
fn test_parse_error(#[files("tests/fixtures/broken/*.yaml")] path: PathBuf) {
    let mut formatter = Ghafmt::new();
    let content = fs::read_to_string(&path).unwrap();
    let doc = Document::from_string(content).expect("Should be valid YAML");
    let result = formatter.format_gha_workflow(doc);
    assert!(
        matches!(result, Err(Error::ParseYaml { .. })),
        "expected ParseYaml error for {}, got: {result:?}",
        path.display()
    );
}
