use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use anyhow::{Context, Result, bail};

use crate::{
    inspect::{self, FindingStatus, InspectionReport},
    readiness, scaffold,
    spec::ProjectSpec,
};

const ENV_IGNORE_BLOCK: &str = "# StackPilot: keep local environment files out of version control\n.env\n.env.*\n!.env.example\n";
const RECIPE_NAME: &str = "base";

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

#[derive(Debug, Clone, PartialEq, Eq)]
struct StackProfile {
    project_name: String,
    language: &'static str,
    framework: &'static str,
}

pub fn run(root: &Path, recipes_dir: &Path, apply: bool) -> Result<()> {
    let plan = plan_repository(root, recipes_dir)?;
    print_plan(&plan, apply);

    if !apply {
        if !plan.changes.is_empty() {
            println!(
                "\nPreview only: no files were changed. Re-run with --apply to write these safe changes."
            );
        }
        return Ok(());
    }

    if plan.changes.is_empty() {
        println!("\nNo safe automatic changes are currently required.");
        return Ok(());
    }

    let applied = apply_plan(&plan)?;
    let after_report = inspect::inspect_repository(&plan.root)?;
    let after = readiness::score(&after_report).total;

    println!("\nApplied {applied} safe change(s).");
    println!("Readiness: {}/100 -> {}/100", plan.readiness_before, after);
    if !plan.deferred.is_empty() {
        println!(
            "{} stack-aware remediation item(s) remain deferred.",
            plan.deferred.len()
        );
    }

    Ok(())
}

pub fn plan_repository(root: &Path, recipes_dir: &Path) -> Result<FixPlan> {
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
    let deferred = plan_stack_fixes(&root, &report, recipes_dir, &mut changes)?;
    plan_metadata_fix(&root, &report, &mut changes)?;

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
            description: "add a safe environment-variable example without secret values"
                .to_string(),
            content: environment_example(root),
        });
    }

    let gitignore = root.join(".gitignore");
    if path_entry_exists(&gitignore) {
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

fn plan_stack_fixes(
    root: &Path,
    report: &InspectionReport,
    recipes_dir: &Path,
    changes: &mut Vec<PlannedChange>,
) -> Result<Vec<DeferredFix>> {
    let docker_needed = finding_status(report, "Docker") != Some(FindingStatus::Passed);
    let ci_needed = finding_status(report, "CI/CD") != Some(FindingStatus::Passed);
    let mut deferred = Vec::new();

    if docker_needed || ci_needed {
        match verified_stack_profile(root, report) {
            Ok(profile) if recipes_dir.is_dir() => {
                let spec = ProjectSpec::configured(
                    profile.project_name,
                    "Backend API".to_string(),
                    profile.language.to_string(),
                    profile.framework.to_string(),
                    "None".to_string(),
                    "None".to_string(),
                    docker_needed,
                    ci_needed,
                    false,
                )?;

                if docker_needed {
                    plan_docker_fix(root, recipes_dir, &spec, changes, &mut deferred)?;
                }
                if ci_needed {
                    plan_ci_fix(root, recipes_dir, &spec, changes, &mut deferred)?;
                }
            }
            Ok(_) => {
                let reason = format!(
                    "StackPilot recipes were not found at {}; pass --recipes-dir or reinstall StackPilot",
                    recipes_dir.display()
                );
                if docker_needed {
                    deferred.push(DeferredFix {
                        control: "Docker",
                        reason: reason.clone(),
                    });
                }
                if ci_needed {
                    deferred.push(DeferredFix {
                        control: "CI/CD",
                        reason,
                    });
                }
            }
            Err(reason) => {
                if docker_needed {
                    deferred.push(DeferredFix {
                        control: "Docker",
                        reason: reason.clone(),
                    });
                }
                if ci_needed {
                    deferred.push(DeferredFix {
                        control: "CI/CD",
                        reason,
                    });
                }
            }
        }
    }

    if finding_status(report, "Terraform") != Some(FindingStatus::Passed) {
        deferred.push(DeferredFix {
            control: "Terraform",
            reason: "infrastructure generation needs an explicit cloud/deployment target rather than guessing"
                .to_string(),
        });
    }
    if finding_status(report, "Health check") != Some(FindingStatus::Passed) {
        deferred.push(DeferredFix {
            control: "Health check",
            reason: "health remediation can touch application routing and needs a stack-specific source adapter"
                .to_string(),
        });
    }

    Ok(deferred)
}

fn plan_docker_fix(
    root: &Path,
    recipes_dir: &Path,
    spec: &ProjectSpec,
    changes: &mut Vec<PlannedChange>,
    deferred: &mut Vec<DeferredFix>,
) -> Result<()> {
    let critical_targets = [Path::new("Dockerfile"), Path::new("compose.yaml")];
    if critical_targets
        .iter()
        .any(|path| path_entry_exists(&root.join(path)))
    {
        deferred.push(DeferredFix {
            control: "Docker",
            reason: "a Docker target already exists but was not recognized; StackPilot will not overwrite or replace it"
                .to_string(),
        });
        return Ok(());
    }

    for (path, description) in [
        (
            Path::new("Dockerfile"),
            "generate the verified golden-path production container definition",
        ),
        (
            Path::new("compose.yaml"),
            "generate the verified local container orchestration definition",
        ),
        (
            Path::new(".dockerignore"),
            "generate container build-context exclusions",
        ),
    ] {
        if path_entry_exists(&root.join(path)) {
            continue;
        }

        changes.push(PlannedChange {
            path: path.to_path_buf(),
            kind: ChangeKind::Create,
            description: description.to_string(),
            content: scaffold::render_destination(spec, RECIPE_NAME, recipes_dir, path)?,
        });
    }

    Ok(())
}

fn plan_ci_fix(
    root: &Path,
    recipes_dir: &Path,
    spec: &ProjectSpec,
    changes: &mut Vec<PlannedChange>,
    deferred: &mut Vec<DeferredFix>,
) -> Result<()> {
    let path = Path::new(".github/workflows/ci.yml");
    let target = root.join(path);

    if path_entry_exists(&target) {
        deferred.push(DeferredFix {
            control: "CI/CD",
            reason: "the StackPilot CI target already exists but was not recognized; StackPilot will not overwrite it"
                .to_string(),
        });
        return Ok(());
    }

    if first_symlink_ancestor(root, path)?.is_some() {
        deferred.push(DeferredFix {
            control: "CI/CD",
            reason: "the .github/workflows path contains a symlink; StackPilot will not write through symlinked directories"
                .to_string(),
        });
        return Ok(());
    }

    changes.push(PlannedChange {
        path: path.to_path_buf(),
        kind: ChangeKind::Create,
        description: "generate the verified golden-path GitHub Actions build/test workflow"
            .to_string(),
        content: scaffold::render_destination(spec, RECIPE_NAME, recipes_dir, path)?,
    });

    Ok(())
}

fn verified_stack_profile(
    root: &Path,
    report: &InspectionReport,
) -> std::result::Result<StackProfile, String> {
    let language = single_value(&report.languages).ok_or_else(|| {
        "automatic Docker/CI generation requires exactly one detected language".to_string()
    })?;
    let framework = single_value(&report.frameworks).ok_or_else(|| {
        "automatic Docker/CI generation requires exactly one detected framework".to_string()
    })?;

    let (language, framework) = match (language, framework) {
        ("Rust", "Axum") => ("Rust", "Axum"),
        ("Go", "Chi") => ("Go", "Chi"),
        ("TypeScript", "NestJS") => ("TypeScript", "NestJS"),
        ("Python", "FastAPI") => ("Python", "FastAPI"),
        ("Java", "Spring Boot") => ("Java", "Spring Boot"),
        ("C#", "ASP.NET Core") => ("C#", "ASP.NET Core"),
        _ => {
            return Err(format!(
                "detected stack {language} + {framework} is not an automatic-remediation golden path"
            ));
        }
    };

    let project_name = match (language, framework) {
        ("Rust", "Axum") => {
            require_file(root, "src/main.rs")?;
            cargo_package_name(root)?
        }
        ("Go", "Chi") => {
            require_file(root, "go.mod")?;
            require_file(root, "main.go")?;
            "app".to_string()
        }
        ("TypeScript", "NestJS") => {
            require_file(root, "package.json")?;
            require_file(root, "tsconfig.json")?;
            require_file(root, "src/main.ts")?;
            let package = read_required_text(root, "package.json")?;
            if !package.contains("\"build\"") || !package.contains("dist/main.js") {
                return Err(
                    "NestJS auto-remediation requires package.json build/start conventions that produce dist/main.js"
                        .to_string(),
                );
            }
            "app".to_string()
        }
        ("Python", "FastAPI") => {
            require_file(root, "pyproject.toml")?;
            require_file(root, "app/main.py")?;
            let pyproject = read_required_text(root, "pyproject.toml")?;
            let main = read_required_text(root, "app/main.py")?;
            if !pyproject.contains("[project.optional-dependencies]")
                || !pyproject.contains("dev =")
                || !main.contains("FastAPI(")
                || !main.contains("app =")
            {
                return Err(
                    "FastAPI auto-remediation requires pyproject dev extras and an app/main.py application named app"
                        .to_string(),
                );
            }
            "app".to_string()
        }
        ("Java", "Spring Boot") => {
            require_file(root, "pom.xml")?;
            let pom = read_required_text(root, "pom.xml")?;
            if !pom.contains("<finalName>app</finalName>") {
                return Err(
                    "Spring Boot Docker remediation requires pom.xml to produce target/app.jar"
                        .to_string(),
                );
            }
            "app".to_string()
        }
        ("C#", "ASP.NET Core") => {
            require_file(root, "app.csproj")?;
            require_file(root, "Program.cs")?;
            "app".to_string()
        }
        _ => unreachable!("golden-path pair matched above"),
    };

    Ok(StackProfile {
        project_name,
        language,
        framework,
    })
}

fn cargo_package_name(root: &Path) -> std::result::Result<String, String> {
    let content = read_required_text(root, "Cargo.toml")?;
    let manifest: toml::Value = toml::from_str(&content)
        .map_err(|error| format!("Cargo.toml could not be parsed safely: {error}"))?;
    manifest
        .get("package")
        .and_then(|package| package.get("name"))
        .and_then(toml::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| {
            "Rust auto-remediation requires a root [package].name in Cargo.toml".to_string()
        })
}

fn require_file(root: &Path, relative: &str) -> std::result::Result<(), String> {
    let path = root.join(relative);
    let metadata = fs::symlink_metadata(&path)
        .map_err(|_| format!("automatic remediation requires {relative}"))?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(format!(
            "automatic remediation requires {relative} to be a regular file"
        ));
    }
    Ok(())
}

fn read_required_text(root: &Path, relative: &str) -> std::result::Result<String, String> {
    require_file(root, relative)?;
    fs::read_to_string(root.join(relative))
        .map_err(|error| format!("failed to read {relative}: {error}"))
}

fn plan_metadata_fix(
    root: &Path,
    report: &InspectionReport,
    changes: &mut Vec<PlannedChange>,
) -> Result<()> {
    let metadata_path = root.join(".stackpilot.toml");
    if path_entry_exists(&metadata_path) {
        return Ok(());
    }

    changes.push(PlannedChange {
        path: PathBuf::from(".stackpilot.toml"),
        kind: ChangeKind::Create,
        description:
            "adopt the repository into StackPilot metadata without changing application code"
                .to_string(),
        content: stackpilot_metadata(root, report, changes),
    });

    Ok(())
}

fn apply_plan(plan: &FixPlan) -> Result<usize> {
    let mut applied = 0;

    for change in &plan.changes {
        ensure_safe_relative_path(&change.path)?;
        ensure_no_symlink_ancestors(&plan.root, &change.path)?;
        let target = plan.root.join(&change.path);

        match change.kind {
            ChangeKind::Create => {
                if path_entry_exists(&target) {
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
                applied += 1;
            }
            ChangeKind::Append => {
                let metadata = fs::symlink_metadata(&target)
                    .with_context(|| format!("failed to inspect {}", target.display()))?;
                if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
                    bail!("refusing to modify non-regular file {}", target.display());
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
                applied += 1;
            }
        }
    }

    Ok(applied)
}

fn ensure_safe_relative_path(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty() || path.is_absolute() {
        bail!("refusing unsafe remediation path: {}", path.display());
    }
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        bail!(
            "refusing remediation path outside repository: {}",
            path.display()
        );
    }
    Ok(())
}

fn ensure_no_symlink_ancestors(root: &Path, relative: &Path) -> Result<()> {
    if let Some(path) = first_symlink_ancestor(root, relative)? {
        bail!(
            "refusing to write through symlinked path {}",
            path.display()
        );
    }
    Ok(())
}

fn first_symlink_ancestor(root: &Path, relative: &Path) -> Result<Option<PathBuf>> {
    let mut current = root.to_path_buf();
    let component_count = relative.components().count();

    for (index, component) in relative.components().enumerate() {
        let Component::Normal(component) = component else {
            continue;
        };
        current.push(component);
        if index + 1 == component_count {
            break;
        }

        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => return Ok(Some(current)),
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("failed to inspect {}", current.display()));
            }
        }
    }

    Ok(None)
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

fn stackpilot_metadata(
    root: &Path,
    report: &InspectionReport,
    changes: &[PlannedChange],
) -> String {
    let name = toml_string(&repository_name(root));
    let detected_language = single_value(&report.languages).unwrap_or("Generic");
    let language = normalized_language(detected_language);
    let framework = normalized_framework(language, single_value(&report.frameworks));
    let docker = finding_status(report, "Docker") == Some(FindingStatus::Passed)
        || planned_create(changes, "Dockerfile")
        || planned_create(changes, "compose.yaml");
    let ci = finding_status(report, "CI/CD") == Some(FindingStatus::Passed)
        || planned_create(changes, ".github/workflows/ci.yml");
    let terraform = finding_status(report, "Terraform") == Some(FindingStatus::Passed);
    let detected_languages = toml_array(&report.languages);
    let detected_frameworks = toml_array(&report.frameworks);

    format!(
        "version = 1

[project]
name = \"{name}\"
kind = \"Generic\"
language = \"{language}\"
framework = \"{framework}\"
database = \"None\"
cloud = \"None\"

[features]
docker = {docker}
ci = {ci}
terraform = {terraform}

[stackpilot]
recipe = \"adopted\"
managed = false

[detected]
languages = {detected_languages}
frameworks = {detected_frameworks}
"
    )
}

fn planned_create(changes: &[PlannedChange], path: &str) -> bool {
    changes
        .iter()
        .any(|change| change.kind == ChangeKind::Create && change.path == Path::new(path))
}

fn normalized_language(language: &str) -> &str {
    match language {
        "Rust" | "Go" | "TypeScript" | "Python" | "Java" | "C#" => language,
        _ => "Generic",
    }
}

fn normalized_framework<'a>(language: &str, framework: Option<&'a str>) -> &'a str {
    match (language, framework) {
        ("Rust", Some("Axum")) => "Axum",
        ("Go", Some("Chi")) => "Chi",
        ("TypeScript", Some("NestJS")) => "NestJS",
        ("Python", Some("FastAPI")) => "FastAPI",
        ("Java", Some("Spring Boot")) => "Spring Boot",
        ("C#", Some("ASP.NET Core")) => "ASP.NET Core",
        _ => "None",
    }
}

fn toml_array(values: &[String]) -> String {
    let values = values
        .iter()
        .map(|value| format!("\"{}\"", toml_string(value)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{values}]")
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

fn path_entry_exists(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
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
    use std::{fs, path::Path};

    use tempfile::tempdir;

    use super::{ChangeKind, apply_plan, plan_repository};

    #[test]
    fn plans_only_safe_foundation_changes_for_unverified_stack() {
        let repo = tempdir().expect("repository");
        fs::write(
            repo.path().join("package.json"),
            r#"{"dependencies":{"@nestjs/core":"latest"},"devDependencies":{"typescript":"latest"}}"#,
        )
        .expect("package manifest");
        fs::write(repo.path().join("tsconfig.json"), "{}").expect("tsconfig");

        let plan = plan_repository(repo.path(), Path::new("recipes")).expect("fix plan");

        assert!(
            plan.changes
                .iter()
                .any(|change| change.path == Path::new(".env.example"))
        );
        assert!(
            plan.changes
                .iter()
                .any(|change| change.path == Path::new(".gitignore"))
        );
        assert!(
            plan.changes
                .iter()
                .any(|change| change.path == Path::new(".stackpilot.toml"))
        );
        assert!(plan.deferred.iter().any(|fix| fix.control == "Docker"));
        assert!(plan.deferred.iter().any(|fix| fix.control == "CI/CD"));
        assert!(plan.deferred.iter().any(|fix| fix.control == "Terraform"));
        assert!(
            plan.deferred
                .iter()
                .any(|fix| fix.control == "Health check")
        );
    }

    #[test]
    fn verified_go_stack_plans_docker_and_ci() {
        let repo = tempdir().expect("repository");
        fs::write(
            repo.path().join("go.mod"),
            "module example.com/payments\n\ngo 1.27\n\nrequire github.com/go-chi/chi/v5 v5.0.0\n",
        )
        .expect("go.mod");
        fs::write(
            repo.path().join("main.go"),
            "package main\nfunc main() {}\n",
        )
        .expect("main.go");

        let plan = plan_repository(repo.path(), Path::new("recipes")).expect("fix plan");

        for path in [
            "Dockerfile",
            "compose.yaml",
            ".dockerignore",
            ".github/workflows/ci.yml",
        ] {
            assert!(
                plan.changes
                    .iter()
                    .any(|change| change.path == Path::new(path)),
                "expected {path} to be planned"
            );
        }
        assert!(!plan.deferred.iter().any(|fix| fix.control == "Docker"));
        assert!(!plan.deferred.iter().any(|fix| fix.control == "CI/CD"));

        let applied = apply_plan(&plan).expect("apply plan");
        assert!(applied >= 4);
        assert!(
            fs::read_to_string(repo.path().join("Dockerfile"))
                .expect("Dockerfile")
                .contains("FROM golang:1.27-alpine AS build")
        );
        assert!(
            fs::read_to_string(repo.path().join(".github/workflows/ci.yml"))
                .expect("CI")
                .contains("go test ./...")
        );
    }

    #[test]
    fn apply_creates_environment_hygiene_without_overwriting_source() {
        let repo = tempdir().expect("repository");
        fs::write(repo.path().join("main.go"), "package main\n").expect("source");
        let plan = plan_repository(repo.path(), Path::new("recipes")).expect("fix plan");

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
        fs::write(
            repo.path().join(".env.example"),
            "CUSTOM_VALUE=placeholder\n",
        )
        .expect("env example");
        fs::write(repo.path().join(".gitignore"), "__pycache__/\n").expect("gitignore");

        let plan = plan_repository(repo.path(), Path::new("recipes")).expect("fix plan");
        let env_change = plan
            .changes
            .iter()
            .find(|change| change.path == Path::new(".env.example"));
        let gitignore_change = plan
            .changes
            .iter()
            .find(|change| change.path == Path::new(".gitignore"))
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
        fs::write(
            repo.path().join("Cargo.toml"),
            "[package]\nname='demo'\nversion='0.1.0'\n",
        )
        .expect("manifest");
        fs::write(repo.path().join(".env.example"), "PORT=3000\n").expect("env example");
        fs::write(repo.path().join(".gitignore"), ".env\n").expect("gitignore");

        let plan = plan_repository(repo.path(), Path::new("recipes")).expect("fix plan");

        assert!(
            !plan.changes.iter().any(|change| {
                matches!(change.path.to_str(), Some(".env.example" | ".gitignore"))
            })
        );
        assert!(
            plan.changes
                .iter()
                .any(|change| change.path == Path::new(".stackpilot.toml"))
        );
    }

    #[cfg(unix)]
    #[test]
    fn refuses_to_plan_ci_through_symlinked_github_directory() {
        use std::os::unix::fs::symlink;

        let repo = tempdir().expect("repository");
        let outside = tempdir().expect("outside");
        fs::write(
            repo.path().join("go.mod"),
            "module example.com/payments\n\ngo 1.27\n\nrequire github.com/go-chi/chi/v5 v5.0.0\n",
        )
        .expect("go.mod");
        fs::write(
            repo.path().join("main.go"),
            "package main\nfunc main() {}\n",
        )
        .expect("main.go");
        symlink(outside.path(), repo.path().join(".github")).expect("symlink .github");

        let plan = plan_repository(repo.path(), Path::new("recipes")).expect("fix plan");

        assert!(
            !plan
                .changes
                .iter()
                .any(|change| change.path == Path::new(".github/workflows/ci.yml"))
        );
        assert!(plan.deferred.iter().any(|fix| fix.control == "CI/CD"));
    }
}
