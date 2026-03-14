use std::path::PathBuf;

use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use rstest::rstest;
use tempfile::TempDir;

#[allow(clippy::expect_used)]
fn cmd() -> Command {
    Command::cargo_bin("ghafmt").expect("ghafmt binary not found")
}

/// Copy a fixture file into a temporary directory and return the dir + new path.
/// The `TempDir` must be kept alive for the duration of the test.
#[allow(clippy::expect_used)]
fn copy_fixture(fixture: &str) -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let filename = PathBuf::from(fixture)
        .file_name()
        .expect("fixture has filename")
        .to_owned();
    let dest = dir.path().join(filename);
    std::fs::copy(fixture, &dest).expect("copy fixture");
    (dir, dest)
}

#[rstest]
fn default_mode_outputs_formatted_yaml_to_stdout() {
    let content = std::fs::read_to_string("tests/fixtures/clean/identity.yaml").unwrap();
    cmd()
        .arg("tests/fixtures/clean/identity.yaml")
        .assert()
        .success()
        .stdout(content);
}

#[test]
fn default_mode_exits_1_for_missing_file() {
    cmd()
        .arg("tests/fixtures/does_not_exist.yaml")
        .assert()
        .failure();
}

#[test]
fn default_mode_exits_1_for_multiple_files_without_write() {
    cmd()
        .arg("tests/fixtures/clean/identity.yaml")
        .arg("tests/fixtures/clean/basic_reorder.yaml")
        .assert()
        .failure()
        .stderr(predicates::str::contains("--write"));
}

#[test]
fn default_mode_stdin_formats_and_writes_to_stdout() {
    let dirty = std::fs::read_to_string("tests/fixtures/dirty/basic_reorder.yaml").unwrap();
    let clean = std::fs::read_to_string("tests/fixtures/clean/basic_reorder.yaml").unwrap();
    cmd()
        .arg("-")
        .write_stdin(dirty)
        .assert()
        .success()
        .stdout(clean);
}

#[rstest]
#[case("tests/fixtures/clean/identity.yaml", true)]
#[case("tests/fixtures/clean/basic_reorder.yaml", true)]
#[case("tests/fixtures/dirty/basic_reorder.yaml", false)]
#[case("tests/fixtures/dirty/step_reorder.yaml", false)]
fn check_mode_exit_code(#[case] path: &str, #[case] expect_success: bool) {
    let assertion = cmd().arg("--check").arg(path).assert();
    if expect_success {
        assertion.success();
    } else {
        assertion.failure();
    }
}

#[rstest]
#[case::single_non_existent_file_fail(vec!["tests/fixtures/does_not_exist.yaml"], false)]
#[case::two_clean_files_succeed(vec!["tests/fixtures/clean/basic_reorder.yaml","tests/fixtures/clean/basic_reorder.yaml"], true)]
#[case::one_dirty_file_fail(vec!["tests/fixtures/clean/basic_reorder.yaml","tests/fixtures/dirty/step_reorder.yaml"], false)]
fn check_mode_exits_correctly(#[case] file_args: Vec<&str>, #[case] expect_success: bool) {
    let mut assertion = cmd().arg("--check").args(file_args).assert();
    assertion = assertion.stdout("");
    if expect_success {
        assertion.success();
    } else {
        assertion.failure();
    }
}

#[rstest]
#[case::success_for_formatted_content("tests/fixtures/clean/identity.yaml", true)]
#[case::fail_for_dirty_content("tests/fixtures/dirty/basic_reorder.yaml", false)]
fn check_mode_stdin(#[case] path: &str, #[case] expect_success: bool) {
    let content = std::fs::read_to_string(path).unwrap();
    let assertion = cmd().arg("--check").arg("-").write_stdin(content).assert();
    if expect_success {
        assertion.success();
    } else {
        assertion.failure();
    }
}

// --- Write mode ---

#[rstest]
#[case::format_file_in_place(vec![("tests/fixtures/dirty/basic_reorder.yaml", "tests/fixtures/clean/basic_reorder.yaml")], true)]
#[case::format_does_not_change_a_formatted_file(vec![("tests/fixtures/clean/identity.yaml", "tests/fixtures/clean/identity.yaml")], true)]
#[case::format_multiple_files(vec![("tests/fixtures/dirty/basic_reorder.yaml", "tests/fixtures/clean/basic_reorder.yaml"), ("tests/fixtures/dirty/step_reorder.yaml", "tests/fixtures/clean/step_reorder.yaml")], true)]
fn write_mode(#[case] inputs_and_outputs: Vec<(&str, &str)>, #[case] expect_success: bool) {
    for (input, output) in &inputs_and_outputs {
        let (_dir, path) = copy_fixture(input);
        let assertion = cmd().arg("--write").arg(&path).assert().success();
        if expect_success {
            assertion.success();
            assert_eq!(
                std::fs::read_to_string(output).unwrap(),
                std::fs::read_to_string(&path).unwrap()
            );
        } else {
            assertion.failure();
        }
    }
}

#[test]
fn write_mode_exits_1_for_missing_file() {
    cmd()
        .arg("--write")
        .arg("tests/fixtures/does_not_exist.yaml")
        .assert()
        .failure();
}

#[test]
fn write_mode_rejects_stdin() {
    cmd()
        .arg("--write")
        .arg("-")
        .assert()
        .failure()
        .stderr(predicates::str::contains("--write"));
}

#[test]
fn write_mode_expands_directory_and_formats_yaml_files() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path1 = dir.path().join("basic_reorder.yaml");
    let path2 = dir.path().join("step_reorder.yaml");
    std::fs::copy("tests/fixtures/dirty/basic_reorder.yaml", &path1).unwrap();
    std::fs::copy("tests/fixtures/dirty/step_reorder.yaml", &path2).unwrap();

    let clean1 = std::fs::read_to_string("tests/fixtures/clean/basic_reorder.yaml").unwrap();
    let clean2 = std::fs::read_to_string("tests/fixtures/clean/step_reorder.yaml").unwrap();

    cmd().arg("--write").arg(dir.path()).assert().success();

    assert_eq!(std::fs::read_to_string(&path1).unwrap(), clean1);
    assert_eq!(std::fs::read_to_string(&path2).unwrap(), clean2);
}

#[test]
fn check_mode_expands_directory() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::copy(
        "tests/fixtures/dirty/basic_reorder.yaml",
        dir.path().join("basic_reorder.yaml"),
    )
    .unwrap();

    cmd().arg("--check").arg(dir.path()).assert().failure();
}

#[test]
fn directory_expansion_ignores_non_yaml_files() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("not_yaml.txt"), "not yaml").unwrap();
    std::fs::write(dir.path().join("also_not.json"), "{}").unwrap();

    // No yaml files → nothing to format → exit 0, no output.
    cmd().arg(dir.path()).assert().success().stdout("");
}

#[test]
fn list_mode_exits_0_and_no_output_for_clean_file() {
    cmd()
        .arg("--list")
        .arg("tests/fixtures/clean/identity.yaml")
        .assert()
        .success()
        .stdout("");
}

#[test]
fn list_mode_exits_1_and_prints_path_for_dirty_file() {
    cmd()
        .arg("--list")
        .arg("tests/fixtures/dirty/basic_reorder.yaml")
        .assert()
        .failure()
        .stdout(predicates::str::contains("basic_reorder.yaml"));
}

#[test]
fn list_mode_only_prints_dirty_files_when_mixed() {
    cmd()
        .arg("--list")
        .arg("tests/fixtures/clean/identity.yaml")
        .arg("tests/fixtures/dirty/basic_reorder.yaml")
        .assert()
        .failure()
        .stdout(predicates::str::contains("basic_reorder.yaml"))
        .stdout(predicates::str::contains("identity.yaml").not());
}

#[test]
fn list_mode_exits_1_for_missing_file() {
    cmd()
        .arg("--list")
        .arg("tests/fixtures/does_not_exist.yaml")
        .assert()
        .failure();
}

#[test]
fn color_never_produces_no_ansi_in_error_output() {
    let output = cmd()
        .arg("--color")
        .arg("never")
        .arg("tests/fixtures/does_not_exist.yaml")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains('\x1b'),
        "expected no ANSI codes, got: {stderr:?}"
    );
}

#[test]
fn no_color_env_var_disables_color() {
    let output = cmd()
        .env("NO_COLOR", "1")
        .arg("tests/fixtures/does_not_exist.yaml")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains('\x1b'),
        "expected no ANSI codes, got: {stderr:?}"
    );
}
