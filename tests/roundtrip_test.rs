use std::{fs::read_to_string, path::PathBuf};

use fyaml::Document;
use ghafmt::Ghafmt;
use patharg::InputArg;
use rstest::rstest;
use similar_asserts::assert_eq;

#[rstest]
fn test_formatter(#[files("tests/fixtures/dirty/**/*.yaml")] path: PathBuf) {
    let components: Vec<_> = path.components().collect();
    let dirty_pos = components
        .iter()
        .rposition(|c| c.as_os_str() == "dirty")
        .expect("test file path must contain a 'dirty' component");
    let relative: PathBuf = components[dirty_pos + 1..].iter().collect();
    let clean_file_path = PathBuf::from("tests/fixtures/clean").join(relative);

    let clean_file_contents = read_to_string(clean_file_path.clone())
        .unwrap_or_else(|_| panic!("{} not found", clean_file_path.display()));

    let mut formatter = Ghafmt::default();

    let content = read_to_string(&path).expect("Could not read test file");
    let doc = Document::from_string(content).expect("Should be valid YAML");

    let (formatted, _) = formatter
        .format_gha_document(doc, &InputArg::Path(path.clone()))
        .expect("Could not format workflow");

    assert_eq!(clean_file_contents, formatted);
}
