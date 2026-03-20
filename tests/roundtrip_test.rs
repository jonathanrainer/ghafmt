use std::{fs::read_to_string, path::PathBuf};

use ghafmt::Ghafmt;
use rstest::rstest;
use similar_asserts::assert_eq;

#[rstest]
fn test_formatter(#[files("tests/fixtures/dirty/*.yaml")] path: PathBuf) {
    let clean_file_name = path
        .file_name()
        .expect("test file path always has a file name")
        .to_str()
        .expect("file name is valid UTF-8")
        .to_owned();
    let clean_file_path = PathBuf::from("tests/fixtures/clean").join(clean_file_name.clone());

    let clean_file_contents = read_to_string(clean_file_path.clone())
        .unwrap_or_else(|_| panic!("{} not found", clean_file_path.display()));

    let mut formatter = Ghafmt::new();

    let content = read_to_string(&path).expect("Could not read test file");
    let (formatted, _) = formatter
        .format_gha_workflow(&content, &clean_file_name)
        .expect("Could not format workflow");

    assert_eq!(clean_file_contents, formatted);
}
