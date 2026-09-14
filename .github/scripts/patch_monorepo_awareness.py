from pathlib import Path


def replace(path: str, old: str, new: str) -> None:
    file = Path(path)
    text = file.read_text()
    if old not in text:
        raise SystemExit(f"expected patch anchor not found in {path}: {old[:80]!r}")
    file.write_text(text.replace(old, new, 1))


def insert_before_last_brace(path: str, block: str) -> None:
    file = Path(path)
    text = file.read_text()
    index = text.rfind("\n}")
    if index < 0:
        raise SystemExit(f"module closing brace not found in {path}")
    file.write_text(text[:index] + "\n" + block.rstrip() + text[index:])


# src/git.rs: deterministic repository-root discovery without shelling out to Git.
replace(
    "src/git.rs",
    "use std::{\n    path::Path,\n    process::{Command, Stdio},\n};",
    "use std::{\n    fs,\n    path::{Path, PathBuf},\n    process::{Command, Stdio},\n};",
)
replace(
    "src/git.rs",
    "pub fn init_repository(directory: &Path) -> Result<()> {",
    """pub fn find_repository_root(path: &Path) -> Option<PathBuf> {
    let start = path.canonicalize().ok()?;
    start
        .ancestors()
        .find(|candidate| fs::symlink_metadata(candidate.join(\".git\")).is_ok())
        .map(Path::to_path_buf)
}

pub fn init_repository(directory: &Path) -> Result<()> {""",
)

# src/inspect.rs: keep application detection target-local and inherit repository-owned controls.
replace(
    "src/inspect.rs",
    "use crate::{deployment, security};",
    "use crate::{deployment, git, security};",
)
replace(
    "src/inspect.rs",
    """pub struct InspectionReport {
    pub root: PathBuf,
    pub languages: Vec<String>,""",
    """pub struct InspectionReport {
    pub root: PathBuf,
    pub repository_root: PathBuf,
    pub languages: Vec<String>,""",
)
replace(
    "src/inspect.rs",
    """    let root = root
        .canonicalize()
        .with_context(|| format!(\"failed to resolve repository path {}\", root.display()))?;
    let files = collect_files(&root)?;
    let languages = detect_languages(&root, &files);
    let frameworks = detect_frameworks(&files);
    let docker = detect_docker(&root, &files);
    let ci = detect_ci(&root, &files);
    let terraform = detect_terraform(&files);
    let health = detect_health_check(&files);
    let environment = detect_environment_hygiene(&root, &files);
    let stackpilot_metadata = files
        .iter()
        .any(|path| path.file_name().and_then(|name| name.to_str()) == Some(\".stackpilot.toml\"));
    let deployment_recommendation =
        deployment::recommendation(&root, &languages, &frameworks, docker.detected);
    let ecs_foundation = deployment::foundation_detected(&root, deployment::AWS_ECS_FARGATE);""",
    """    let root = root
        .canonicalize()
        .with_context(|| format!(\"failed to resolve repository path {}\", root.display()))?;
    let repository_root = git::find_repository_root(&root).unwrap_or_else(|| root.clone());
    let nested_target = repository_root != root;
    let files = collect_files(&root)?;
    let repository_files = if nested_target {
        collect_files(&repository_root)?
    } else {
        files.clone()
    };
    let languages = detect_languages(&root, &files);
    let frameworks = detect_frameworks(&files);
    let docker = detect_docker(&root, &files);
    let mut ci = detect_ci(&repository_root, &repository_files);
    if nested_target && ci.detected {
        ci.detail = format!(\"Inherited from repository root: {}\", ci.detail);
    }
    let terraform = detect_terraform(&files);
    let health = detect_health_check(&files);
    let mut environment = detect_environment_hygiene(&root, &files);
    if nested_target && environment.status == FindingStatus::Missing {
        let inherited = detect_environment_hygiene(&repository_root, &repository_files);
        if inherited.status != FindingStatus::Missing {
            environment = EnvironmentDetection {
                status: inherited.status,
                detail: format!(\"Inherited from repository root: {}\", inherited.detail),
                recommendation: inherited.recommendation,
            };
        }
    }
    let stackpilot_metadata = root.join(\".stackpilot.toml\").is_file();
    let heterogeneous_root = !nested_target && (languages.len() > 1 || frameworks.len() > 1);
    let deployment_recommendation =
        deployment::recommendation(&root, &languages, &frameworks, docker.detected);
    let ecs_foundation = deployment::foundation_detected(&root, deployment::AWS_ECS_FARGATE);""",
)
replace(
    "src/inspect.rs",
    """        Finding {
            category: \"Configuration\",
            name: \"StackPilot metadata\",
            status: if stackpilot_metadata {
                FindingStatus::Passed
            } else {
                FindingStatus::Warning
            },
            detail: if stackpilot_metadata {
                \".stackpilot.toml detected\".to_string()
            } else {
                \".stackpilot.toml not found\".to_string()
            },
            recommendation: (!stackpilot_metadata).then(|| {
                \"Add StackPilot project metadata so future remediation and upgrades can track repository intent.\"
                    .to_string()
            }),
        },""",
    """        Finding {
            category: \"Configuration\",
            name: \"StackPilot metadata\",
            status: if stackpilot_metadata {
                FindingStatus::Passed
            } else {
                FindingStatus::Warning
            },
            detail: if stackpilot_metadata {
                \".stackpilot.toml detected\".to_string()
            } else if heterogeneous_root {
                \".stackpilot.toml not found; heterogeneous repository detected\".to_string()
            } else {
                \".stackpilot.toml not found\".to_string()
            },
            recommendation: (!stackpilot_metadata).then(|| {
                if heterogeneous_root {
                    \"Inspect a supported service path before adopting StackPilot metadata in a heterogeneous repository.\"
                        .to_string()
                } else {
                    \"Add StackPilot project metadata so future remediation and upgrades can track repository intent.\"
                        .to_string()
                }
            }),
        },""",
)
replace(
    "src/inspect.rs",
    """    findings.extend(security::inspect(&root)?);

    Ok(InspectionReport {
        root,
        languages,""",
    """    if nested_target {
        findings.push(Finding {
            category: \"Scope\",
            name: \"Repository scope\",
            status: FindingStatus::Passed,
            detail: format!(
                \"Nested target detected; repository-level controls are inherited from {}\",
                repository_root.display()
            ),
            recommendation: None,
        });
    } else if heterogeneous_root {
        findings.push(Finding {
            category: \"Scope\",
            name: \"Repository scope\",
            status: FindingStatus::Warning,
            detail: \"Multiple languages or frameworks detected; readiness is an aggregate repository view, not a single-service readiness score\"
                .to_string(),
            recommendation: Some(
                \"Inspect a specific service path for service-level readiness and remediation.\"
                    .to_string(),
            ),
        });
    }
    findings.extend(security::inspect_scoped(&repository_root, &root)?);

    Ok(InspectionReport {
        root,
        repository_root,
        languages,""",
)

# src/security.rs: workflows, lockfiles and dependency automation are repository-scoped;
# container presence remains target-scoped.
replace(
    "src/security.rs",
    """pub fn inspect(root: &Path) -> Result<Vec<Finding>> {
    let workflow_text = read_workflows(root)?;""",
    """pub fn inspect(root: &Path) -> Result<Vec<Finding>> {
    inspect_scoped(root, root)
}

pub fn inspect_scoped(repository_root: &Path, target_root: &Path) -> Result<Vec<Finding>> {
    let workflow_text = read_workflows(repository_root)?;""",
)
replace(
    "src/security.rs",
    ".filter(|name| root.join(name).is_file())",
    ".filter(|name| repository_root.join(name).is_file())",
)
replace(
    "src/security.rs",
    """    let dependabot = root.join(\".github/dependabot.yml\").is_file()
        || root.join(\".github/dependabot.yaml\").is_file()
        || root.join(\"renovate.json\").is_file()
        || root.join(\"renovate.json5\").is_file()
        || root.join(\".renovaterc\").is_file()
        || root.join(\".renovaterc.json\").is_file();""",
    """    let dependabot = repository_root.join(\".github/dependabot.yml\").is_file()
        || repository_root.join(\".github/dependabot.yaml\").is_file()
        || repository_root.join(\"renovate.json\").is_file()
        || repository_root.join(\"renovate.json5\").is_file()
        || repository_root.join(\".renovaterc\").is_file()
        || repository_root.join(\".renovaterc.json\").is_file();""",
)
replace(
    "src/security.rs",
    "let workflow_permissions = inspect_workflow_permissions(root)?;",
    "let workflow_permissions = inspect_workflow_permissions(repository_root)?;",
)
replace(
    "src/security.rs",
    "} else if root.join(\"Dockerfile\").is_file() {",
    "} else if target_root.join(\"Dockerfile\").is_file() {",
)
replace(
    "src/security.rs",
    "\"No container image scan detected; repository does not expose a root Dockerfile\"",
    "\"No container image scan detected; target does not expose a Dockerfile\"",
)

# src/readiness.rs: make scope visible in human output and update test fixture construction.
replace(
    "src/readiness.rs",
    """    println!(\"Repository: {}\", report.root.display());""",
    """    if report.repository_root != report.root {
        println!(\"Target: {}\", report.root.display());
        println!(\"Repository root: {}\", report.repository_root.display());
    } else {
        println!(\"Repository: {}\", report.root.display());
    }""",
)
replace(
    "src/readiness.rs",
    """        InspectionReport {
            root: PathBuf::from(\"/tmp/example\"),
            languages:""",
    """        InspectionReport {
            root: PathBuf::from(\"/tmp/example\"),
            repository_root: PathBuf::from(\"/tmp/example\"),
            languages:""",
)

# src/fix.rs: defend repository-scoped write locations and defer heterogeneous root adoption.
replace(
    "src/fix.rs",
    """    deployment,
    inspect::{self, FindingStatus, InspectionReport},""",
    """    deployment, git,
    inspect::{self, FindingStatus, InspectionReport},""",
)
replace(
    "src/fix.rs",
    """    let readiness_before = readiness::score(&report).total;
    let mut changes = Vec::new();
    let mut updates = Vec::new();

    plan_environment_fixes(&root, &report, &mut changes)?;
    if security_baseline {
        plan_security_baseline(&root, &report, recipes_dir, &mut changes)?;
    }""",
    """    let readiness_before = readiness::score(&report).total;
    let mut changes = Vec::new();
    let mut updates = Vec::new();
    let mut deferred = Vec::new();

    plan_environment_fixes(&root, &report, &mut changes)?;
    if security_baseline {
        plan_security_baseline(&root, &report, recipes_dir, &mut changes, &mut deferred)?;
    }""",
)
replace(
    "src/fix.rs",
    """    let mut deferred = plan_stack_fixes(
        &root,
        &report,
        recipes_dir,
        cloud,
        &mut changes,
        &mut updates,
    )?;""",
    """    let mut stack_deferred = plan_stack_fixes(
        &root,
        &report,
        recipes_dir,
        cloud,
        &mut changes,
        &mut updates,
    )?;
    deferred.append(&mut stack_deferred);""",
)
replace(
    "src/fix.rs",
    "plan_metadata_fix(&root, &report, cloud, &mut changes)?;",
    "plan_metadata_fix(&root, &report, cloud, &mut changes, &mut deferred)?;",
)
replace(
    "src/fix.rs",
    """fn plan_security_baseline(
    root: &Path,
    report: &InspectionReport,
    recipes_dir: &Path,
    changes: &mut Vec<PlannedChange>,
) -> Result<()> {
    let profile = match verified_stack_profile(root, report) {""",
    """fn plan_security_baseline(
    root: &Path,
    report: &InspectionReport,
    recipes_dir: &Path,
    changes: &mut Vec<PlannedChange>,
    deferred: &mut Vec<DeferredFix>,
) -> Result<()> {
    if report.repository_root != root {
        deferred.push(DeferredFix {
            control: \"Security baseline\",
            reason: format!(
                \"security workflows and dependency automation are repository-scoped; run `stackpilot fix {} --security` from the repository root\",
                report.repository_root.display()
            ),
        });
        return Ok(());
    }

    let profile = match verified_stack_profile(root, report) {""",
)
replace(
    "src/fix.rs",
    """fn plan_ci_fix(
    root: &Path,
    recipes_dir: &Path,
    spec: &ProjectSpec,
    changes: &mut Vec<PlannedChange>,
    deferred: &mut Vec<DeferredFix>,
) -> Result<()> {
    let path = Path::new(\".github/workflows/ci.yml\");""",
    """fn plan_ci_fix(
    root: &Path,
    recipes_dir: &Path,
    spec: &ProjectSpec,
    changes: &mut Vec<PlannedChange>,
    deferred: &mut Vec<DeferredFix>,
) -> Result<()> {
    if let Some(repository_root) = git::find_repository_root(root)
        && repository_root != root
    {
        deferred.push(DeferredFix {
            control: \"CI/CD\",
            reason: format!(
                \"GitHub Actions is repository-scoped; StackPilot will not create a nested .github/workflows directory under this service. Repository root: {}\",
                repository_root.display()
            ),
        });
        return Ok(());
    }

    let path = Path::new(\".github/workflows/ci.yml\");""",
)
replace(
    "src/fix.rs",
    """fn plan_metadata_fix(
    root: &Path,
    report: &InspectionReport,
    cloud: Option<&str>,
    changes: &mut Vec<PlannedChange>,
) -> Result<()> {
    let metadata_path = root.join(\".stackpilot.toml\");
    if path_entry_exists(&metadata_path) {
        return Ok(());
    }

    changes.push(PlannedChange {""",
    """fn plan_metadata_fix(
    root: &Path,
    report: &InspectionReport,
    cloud: Option<&str>,
    changes: &mut Vec<PlannedChange>,
    deferred: &mut Vec<DeferredFix>,
) -> Result<()> {
    let metadata_path = root.join(\".stackpilot.toml\");
    if path_entry_exists(&metadata_path) {
        return Ok(());
    }

    if report.repository_root == root && (report.languages.len() > 1 || report.frameworks.len() > 1)
    {
        deferred.push(DeferredFix {
            control: \"StackPilot metadata\",
            reason: \"heterogeneous repository detected; inspect and adopt a specific supported service path instead of assigning one golden-path identity to the whole repository\"
                .to_string(),
        });
        return Ok(());
    }

    changes.push(PlannedChange {""",
)
replace(
    "src/fix.rs",
    """    println!(\"StackPilot fix\");
    println!(\"Repository: {}\", plan.root.display());
    println!(\"Mode: {}\", if apply { \"apply\" } else { \"preview\" });""",
    """    println!(\"StackPilot fix\");
    if let Some(repository_root) = git::find_repository_root(&plan.root)
        && repository_root != plan.root
    {
        println!(\"Target: {}\", plan.root.display());
        println!(\"Repository root: {}\", repository_root.display());
    } else {
        println!(\"Repository: {}\", plan.root.display());
    }
    println!(\"Mode: {}\", if apply { \"apply\" } else { \"preview\" });""",
)

# Regression tests for the real field-validation shape.
insert_before_last_brace(
    "src/inspect.rs",
    r'''
    #[test]
    fn nested_service_inherits_repository_owned_controls_without_cross_service_runtime_detection() {
        let repo = tempdir().expect("repository");
        fs::create_dir(repo.path().join(".git")).expect("git marker");
        fs::create_dir_all(repo.path().join(".github/workflows")).expect("workflows");
        fs::write(repo.path().join(".github/workflows/ci.yml"), "name: CI\npermissions:\n  contents: read\n")
            .expect("workflow");
        fs::write(repo.path().join(".env.example"), "PORT=3000\n").expect("env example");
        fs::write(repo.path().join(".gitignore"), ".env\n").expect("gitignore");
        fs::write(repo.path().join("pnpm-lock.yaml"), "lockfileVersion: '9.0'\n").expect("lockfile");

        let service = repo.path().join("services/api");
        fs::create_dir_all(service.join("src")).expect("service source");
        fs::write(
            service.join("package.json"),
            r#"{"dependencies":{"@nestjs/core":"latest"},"devDependencies":{"typescript":"latest"}}"#,
        )
        .expect("package.json");
        fs::write(service.join("tsconfig.json"), "{}").expect("tsconfig");
        fs::write(service.join("src/main.ts"), "app.get('/health', () => ({ status: 'ok' }));")
            .expect("source");

        let report = inspect_repository(&service).expect("inspection");

        assert_eq!(report.repository_root, repo.path().canonicalize().expect("repo root"));
        assert_eq!(report.languages, vec!["TypeScript".to_string()]);
        assert_eq!(report.frameworks, vec!["NestJS".to_string()]);
        let ci = report.finding("CI/CD").expect("CI finding");
        assert_eq!(ci.status, FindingStatus::Passed);
        assert!(ci.detail.contains("Inherited from repository root"));
        let environment = report.finding("Environment config").expect("environment finding");
        assert_eq!(environment.status, FindingStatus::Passed);
        assert!(environment.detail.contains("Inherited from repository root"));
        assert_eq!(
            report.finding("Dependency lockfile").expect("lockfile").status,
            FindingStatus::Passed
        );
        assert_eq!(
            report.finding("Repository scope").expect("scope").status,
            FindingStatus::Passed
        );
    }
''',
)

insert_before_last_brace(
    "src/fix.rs",
    r'''
    #[test]
    fn nested_service_never_plans_repository_scoped_ci_or_environment_files() {
        let repo = tempdir().expect("repository");
        fs::create_dir(repo.path().join(".git")).expect("git marker");
        fs::create_dir_all(repo.path().join(".github/workflows")).expect("workflows");
        fs::write(repo.path().join(".github/workflows/ci.yml"), "name: CI\n")
            .expect("workflow");
        fs::write(repo.path().join(".env.example"), "PORT=3000\n").expect("env example");
        fs::write(repo.path().join(".gitignore"), ".env\n").expect("gitignore");

        let service = repo.path().join("services/api");
        fs::create_dir_all(service.join("src")).expect("service source");
        fs::write(
            service.join("package.json"),
            r#"{"name":"@demo/api","scripts":{"build":"tsc","start":"node dist/main.js"},"dependencies":{"@nestjs/core":"latest","@nestjs/platform-express":"latest"},"devDependencies":{"typescript":"latest"}}"#,
        )
        .expect("package.json");
        fs::write(service.join("tsconfig.json"), "{}").expect("tsconfig");
        fs::write(service.join("src/main.ts"), "app.get('/health', () => ({ status: 'ok' }));")
            .expect("source");

        let plan = plan_repository(&service, Path::new("recipes"), None).expect("fix plan");

        assert!(!plan.changes.iter().any(|change| change.path == Path::new(".github/workflows/ci.yml")));
        assert!(!plan.changes.iter().any(|change| change.path == Path::new(".env.example")));
        assert!(!plan.changes.iter().any(|change| change.path == Path::new(".gitignore")));
        assert!(plan.changes.iter().any(|change| change.path == Path::new("Dockerfile")));
        assert!(plan.changes.iter().any(|change| change.path == Path::new(".stackpilot.toml")));
    }

    #[test]
    fn heterogeneous_repository_root_defers_stackpilot_metadata_adoption() {
        let repo = tempdir().expect("repository");
        fs::create_dir(repo.path().join(".git")).expect("git marker");
        fs::write(
            repo.path().join("package.json"),
            r#"{"dependencies":{"next":"latest"},"devDependencies":{"typescript":"latest"}}"#,
        )
        .expect("package.json");
        fs::write(repo.path().join("tsconfig.json"), "{}").expect("tsconfig");
        let rust = repo.path().join("services/rust-api");
        fs::create_dir_all(rust.join("src")).expect("rust service");
        fs::write(
            rust.join("Cargo.toml"),
            "[package]\nname='rust-api'\nversion='0.1.0'\n[dependencies]\naxum='0.8'\n",
        )
        .expect("Cargo.toml");
        fs::write(rust.join("src/main.rs"), "fn main() {}\n").expect("source");

        let plan = plan_repository(repo.path(), Path::new("recipes"), None).expect("fix plan");

        assert!(!plan.changes.iter().any(|change| change.path == Path::new(".stackpilot.toml")));
        assert!(plan.deferred.iter().any(|fix| fix.control == "StackPilot metadata"));
    }
''',
)
