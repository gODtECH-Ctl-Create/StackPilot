use std::{fs, process::Command};

use tempfile::tempdir;

fn run_stackpilot(path: &std::path::Path, report: &std::path::Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_stackpilot"))
        .args([
            "inspect",
            path.to_str().expect("UTF-8 repository path"),
            "--steward-report",
            report.to_str().expect("UTF-8 report path"),
        ])
        .output()
        .expect("run stackpilot inspect")
}

#[test]
fn inspect_displays_steward_health_without_changing_readiness() {
    let repository = tempdir().expect("repository directory");
    fs::write(repository.path().join("README.md"), "# example\n").expect("write README");

    let report = repository.path().join("steward.json");
    fs::write(
        &report,
        r#"{
            "schemaVersion": 1,
            "tool": "gODtECH Steward",
            "version": 1,
            "root": "/tmp/example",
            "scannedFiles": 1,
            "textFiles": 1,
            "findings": [],
            "healthScore": 96,
            "durationMs": 8,
            "git": {"isRepository": false},
            "categoryCounts": {},
            "ruleCounts": {},
            "rulePacks": [{"id":"core","version":2}]
        }"#,
    )
    .expect("write Steward report");

    let normal = Command::new(env!("CARGO_BIN_EXE_stackpilot"))
        .args(["inspect", repository.path().to_str().expect("UTF-8 path")])
        .output()
        .expect("run normal inspect");
    assert!(normal.status.success());

    let with_steward = run_stackpilot(repository.path(), &report);
    assert!(with_steward.status.success());

    let normal_stdout = String::from_utf8(normal.stdout).expect("UTF-8 normal output");
    let steward_stdout = String::from_utf8(with_steward.stdout).expect("UTF-8 Steward output");

    let normal_readiness = normal_stdout
        .lines()
        .find(|line| line.starts_with("StackPilot Readiness:"))
        .expect("normal readiness line");
    let steward_readiness = steward_stdout
        .lines()
        .find(|line| line.starts_with("StackPilot Readiness:"))
        .expect("Steward readiness line");

    assert_eq!(normal_readiness, steward_readiness);
    assert!(steward_stdout.contains("Steward repository health"));
    assert!(steward_stdout.contains("Health: 96/100"));
    assert!(steward_stdout.contains("Findings: 0"));
    assert!(steward_stdout.contains("Rule packs: core@2"));
}

#[test]
fn inspect_rejects_incompatible_steward_schema() {
    let repository = tempdir().expect("repository directory");
    fs::write(repository.path().join("README.md"), "# example\n").expect("write README");

    let report = repository.path().join("steward.json");
    fs::write(
        &report,
        r#"{
            "schemaVersion": 2,
            "tool": "gODtECH Steward",
            "version": 1,
            "healthScore": 100,
            "findings": []
        }"#,
    )
    .expect("write incompatible report");

    let output = run_stackpilot(repository.path(), &report);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("UTF-8 error output");
    assert!(stderr.contains("unsupported Steward schemaVersion 2"));
}
