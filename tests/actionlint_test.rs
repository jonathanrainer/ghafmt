use std::{path::PathBuf, process::Command};

use rstest::rstest;

#[rstest]
fn test_actionlint_compliance(
    #[files("tests/fixtures/dirty/*.yaml")]
    #[files("tests/fixtures/clean/*.yaml")]
    path: PathBuf,
) {
    let output = Command::new("actionlint")
        .arg("-ignore")
        .arg("could not read reusable workflow file")
        .arg(&path)
        .output()
        .expect("actionlint not found — install it with: brew install actionlint");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success() && stdout.is_empty(),
        "actionlint failed for {}:\n{stdout}{stderr}",
        path.display()
    );
}
