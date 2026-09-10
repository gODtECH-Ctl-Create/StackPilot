use std::{fs, path::{Path, PathBuf}};

use anyhow::{Context, Result, bail};

use crate::{
    inspect::{self, FindingStatus, InspectionReport},
    readiness,
};

const ENV_IGNORE_BLOCK: &str = "# StackPilot: keep local environment files out of version control\n.env\n.env.*\n!.env.example\n";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    Create,
    Append,
}

impl ChangeKind {
    fn symbol(self) -> &'static str {
        match self {
            Self::Create => "+",
            Self::Append => "~",
        }
    }

    fn verb(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Append => "append",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedChange {
    pub path: PathBuf,
    pub kind: ChangeKind,
    pub description: String,
    content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeferredFix {
    pub control: &'static str,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixPlan {
    pub root: PathBuf,
    pub readiness_before: u8,
    pub changes: Vec<PlannedChange>,
    pub deferred: Vec<DeferredFix>,
}

pub fn run(root: &Path, apply: bool) -> Result<()> {
    let plan = plan_repository(root)?;
    print_plan(&plan, apply);

    if !apply {
        if !plan.changes.is_empty() {
            println!("\nPreview only: no files were changed. Re-run with --apply to write these safe changes.");
        }
        return Ok(());
    }

    if plan.changes.is_empty() {
        println!("\nNo safe automatic changes are currently required.");
        return Ok(());
    }

    apply_plan(&plan)?;
    let after_report = inspect::inspect_repository(&plan.root)?;
    let after = readiness::score(&after_report).total;

    println!("\nApplied {} safe change(s).", plan.changes.len());
    println!("Readiness: {}/100 -> {}/100", plan.readiness_before, after);
    if !plan.deferred.is_empty() {
        println!(
            "{} stack-aware remediation item(s) remain deferred.",
            plan.deferred.len()
        );
    }

    Ok(())
}

pub fn plan_repository(root: &Path) -> Result<FixPlan> {
    if !root.exists() {
        bail!("repository path does not exist: {}", root.display());
    }
    if !root.is_dir() {
        bail!("repository path is not a directory: {}", root.display());
    }

    let root = root
        .canonicalize()
        .with_context(|| format!("failed to resolve repository path {}", root.display()))?;
    let report = inspect::inspect_repository(&root)?;
    let readiness_before = readiness::score(&report).total;
    let mut changes = Vec::new();

    plan_environment_fixes(&root, &report, &mut changes)?;
    plan_metadata_fix(&root, &report, &mut changes)?;

    let deferred = deferred_stack_fixes(&report);

    Ok(FixPlan {
        root,
        readiness_before,
        changes,
        deferred,
    })
}

fn plan_environment_fixes(
    root: &Path,
    report: &InspectionReport,
    changes: &mut Vec<PlannedChange>,
) -> Result<()> {
    if finding_status(report, "Environment config") == Some(FindingStatus::Passed) {
        return Ok(());
    }

    let safe_example_exists = [
        ".env.example",
        ".env.sample",
        ".env.template",
        "env.example",
    ]
    .iter()
    .any(|name| root.join(name).is_file());

    if !safe_example_exists {
        changes.push(PlannedChange {
            path: PathBuf::from(".env.example"),
            kind: ChangeKind::Create,
            description: "add a safe environment-variable example without secret values".to_string(),
            content: environment_example(root),
        });
    }

    let gitignore = root.join(".gitignore");
    if gitignore.exists() {
        let metadata = fs::symlink_metadata(&gitignore)
            .with_context(|| format!("failed to inspect {}", gitignore.display()))?;
        if metadata.file_type().is_symlink() {
            return Ok(());
        }

        let content = fs::read_to_string(&gitignore)
            .with_context(|| format!("failed to read {}", gitignore.display()))?;
        if !gitignore_protects_env(&content) {
            changes.push(PlannedChange {
                path: PathBuf::from(".gitignore"),
                kind: ChangeKind::Append,
                description: "protect local .env files while keeping .env.example commit-safe"
                    .to_string(),
                content: format!("\n{ENV_IGNORE_BLOCK}"),
            });
        }
    } else {
        changes.push(PlannedChange {
            path: PathBuf::from(".gitignore"),
            kind: ChangeKind::Create,
            description: "protect local .env files while keeping .env.example commit-safe"
                .to_string(),
            content: ENV_IGNORE_BLOCK.to_string(),
        });
    }

    Ok(())
}

fn plan_metadata_fix(
    root: &Path,
    report: &InspectionReport,
    changes: &mut Vec<PlannedChange>,
) -> Result<()> {
    let metadata_path = root.join(".stackpilot.toml");
    if metadata_path.exists() {
        return Ok(());
    }

    changes.push(PlannedChange {
        path: PathBuf::from(".stackpilot.toml"),
        kind: ChangeKind::Create,
        description: "adopt the repository into StackPilot metadata without changing application code"
            .to_string(),
        content: stackpilot_metadata(root, report)?,
    });

    Ok(())
}

fn deferred_stack_fixes(report: &InspectionReport) -> Vec<DeferredFix> {
    [
        (
            "Docker",
            "container generation needs a verified runtime entrypoint before StackPilot can write it safely",
        ),
        (
            "CI/CD",
            "CI generation needs a verified build/test command adapter for the detected stack",
        ),
        (
            "Terraform",
            "infrastructure generation needs an explicit cloud/deployment target rather than guessing",
        ),
        (
            "Health check",
            "health remediation can touch application routing and needs a stack-specific source adapter",
        ),
    ]
    .into_iter()
    .filter(|(control, _)| finding_status(report, control) != Some(FindingStatus::Passed))
    .map(|(control, reason)| DeferredFix {
        control,
        reason: reason.to_string(),
    })
    .collect()
}

fn apply_plan(plan: &FixPlan) -> Result<()> {
    for change in &plan.changes {
        let target = plan.root.join(&change.path);
        match change.kind {
            ChangeKind::Create => {
                if target.exists() {
                    bail!(
                        "refusing to overwrite {} because it appeared after the fix plan was created",
                        target.display()
                    );
                }
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent).with_context(|| {
                        format!("failed to create parent directory {}", parent.display())
                    })?;
                }
                fs::write(&target, &change.content)
                    .with_context(|| format!("failed to write {}", target.display()))?;
            }
            ChangeKind::Append => {
                let metadata = fs::symlink_metadata(&target)
                    .with_context(|| format!("failed to inspect {}", target.display()))?;
                if metadata.file_type().is_symlink() {
                    bail!("refusing to modify symlinked file {}", target.display());
                }
                let current = fs::read_to_string(&target)
                    .with_context(|| format!("failed to read {}", target.display()))?;
                if gitignore_protects_env(&current) {
                    continue;
                }
                let mut updated = current;
                updated.push_str(&change.content);
                fs::write(&target, updated)
                    .with_context(|| format!("failed to update {}", target.display()))?;
            }
        }
    }

    Ok(())
}

fn print_plan(plan: &FixPlan, apply: bool) {
    println!("StackPilot fix");
    println!("Repository: {}", plan.root.display());
    println!("Mode: {}", if apply { "apply" } else { "preview" });
    println!("Readiness before: {}/100", plan.readiness_before);

    println!("\nSafe automatic changes");
    if plan.changes.is_empty() {
        println!("  none");
    } else {
        for change in &plan.changes {
            println!(
                "{} {} {} — {}",
                change.kind.symbol(),
                change.kind.verb(),
                change.path.display(),
                change.description
            );
        }
    }

    println!("\nDeferred stack-aware changes");
    if plan.deferred.is_empty() {
        println!("  none");
    } else {
        for deferred in &plan.deferred {
            println!("! {} — {}", deferred.control, deferred.reason);
        }
    }
}

fn environment_example(root: &Path) -> String {
    let name = repository_name(root);
    format!(
        "# Safe development defaults. Add variable names here, never real secrets.\nAPP_NAME={}\nAPP_ENV=development\nPORT=3000\n",
        env_value(&name)
    )
}

fn stackpilot_metadata(root: &Path, report: &InspectionReport) -> Result<String> {
    let name = toml_string(&repository_name(root));
    let language = toml_string(single_value(&report.languages).unwrap_or("Unknown"));
    let framework = toml_string(single_value(&report.frameworks).unwrap_or("Unknown"));
    let docker = finding_status(report, "Docker") == Some(FindingStatus::Passed);
    let ci = finding_status(report, "CI/CD") == Some(FindingStatus::Passed);
    let terraform = finding_status(report, "Terraform") == Some(FindingStatus::Passed);

    Ok(format!(
        "version = 1\n\n[project]\nname = \"{name}\"\nkind = \"Existing repository\"\nlanguage = \"{language}\"\nframework = \"{framework}\"\ndatabase = \"Unknown\"\ncloud = \"Unknown\"\n\n[features]\ndocker = {docker}\nci = {ci}\nterraform = {terraform}\n\n[stackpilot]\nrecipe = \"adopted\"\nmanaged = false\n"
    ))
}

fn repository_name(root: &Path) -> String {
    root.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("project")
        .to_string()
}

fn single_value(values: &[String]) -> Option<&str> {
    (values.len() == 1).then(|| values[0].as_str())
}

fn finding_status(report: &InspectionReport, name: &str) -> Option<FindingStatus> {
    report
        .findings
        .iter()
        .find(|finding| finding.name == name)
        .map(|finding| finding.status)
}

fn gitignore_protects_env(content: &str) -> bool {
    content.lines().any(|line| {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('!') {
            return false;
        }
        matches!(line, ".env" | ".env*" | "*.env" | "**/.env") || line.ends_with("/.env")
    })
}

fn toml_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn env_value(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{ChangeKind, apply_plan, plan_repository};

    #[test]
    fn plans_only_safe_foundation_changes() {
        let repo = tempdir().expect("repository");
        fs::write(
            repo.path().join("package.json"),
            r#"{"dependencies":{"@nestjs/core":"latest"},"devDependencies":{"typescript":"latest"}}"#,
        )
        .expect("package manifest");
        fs::write(repo.path().join("tsconfig.json"), "{}").expect("tsconfig");

        let plan = plan_repository(repo.path()).expect("fix plan");

        assert!(plan.changes.iter().any(|change| change.path == std::path::Path::new(".env.example")));
        assert!(plan.changes.iter().any(|change| change.path == std::path::Path::new(".gitignore")));
        assert!(plan.changes.iter().any(|change| change.path == std::path::Path::new(".stackpilot.toml")));
        assert!(plan.deferred.iter().any(|fix| fix.control == "Docker"));
        assert!(plan.deferred.iter().any(|fix| fix.control == "CI/CD"));
        assert!(plan.deferred.iter().any(|fix| fix.control == "Terraform"));
        assert!(plan.deferred.iter().any(|fix| fix.control == "Health check"));
    }

    #[test]
    fn apply_creates_environment_hygiene_without_overwriting_source() {
        let repo = tempdir().expect("repository");
        fs::write(repo.path().join("main.go"), "package main\n").expect("source");
        let plan = plan_repository(repo.path()).expect("fix plan");

        apply_plan(&plan).expect("apply plan");

        assert!(repo.path().join(".env.example").is_file());
        assert!(repo.path().join(".stackpilot.toml").is_file());
        assert!(
            fs::read_to_string(repo.path().join(".gitignore"))
                .expect("gitignore")
                .contains(".env")
        );
        assert_eq!(
            fs::read_to_string(repo.path().join("main.go")).expect("source"),
            "package main\n"
        );
    }

    #[test]
    fn preserves_existing_example_and_appends_gitignore_only_when_needed() {
        let repo = tempdir().expect("repository");
        fs::write(repo.path().join("requirements.txt"), "fastapi\n").expect("requirements");
        fs::write(repo.path().join(".env.example"), "CUSTOM_VALUE=placeholder\n")
            .expect("env example");
        fs::write(repo.path().join(".gitignore"), "__pycache__/\n").expect("gitignore");

        let plan = plan_repository(repo.path()).expect("fix plan");
        let env_change = plan
            .changes
            .iter()
            .find(|change| change.path == std::path::Path::new(".env.example"));
        let gitignore_change = plan
            .changes
            .iter()
            .find(|change| change.path == std::path::Path::new(".gitignore"))
            .expect("gitignore change");

        assert!(env_change.is_none());
        assert_eq!(gitignore_change.kind, ChangeKind::Append);

        apply_plan(&plan).expect("apply plan");
        assert_eq!(
            fs::read_to_string(repo.path().join(".env.example")).expect("env example"),
            "CUSTOM_VALUE=placeholder\n"
        );
    }

    #[test]
    fn no_environment_change_when_hygiene_already_passes() {
        let repo = tempdir().expect("repository");
        fs::write(repo.path().join("Cargo.toml"), "[package]\nname='demo'\nversion='0.1.0'\n")
            .expect("manifest");
        fs::write(repo.path().join(".env.example"), "PORT=3000\n").expect("env example");
        fs::write(repo.path().join(".gitignore"), ".env\n").expect("gitignore");

        let plan = plan_repository(repo.path()).expect("fix plan");

        assert!(!plan.changes.iter().any(|change| {
            matches!(change.path.to_str(), Some(".env.example" | ".gitignore"))
        }));
        assert!(plan.changes.iter().any(|change| change.path == std::path::Path::new(".stackpilot.toml")));
    }
}
