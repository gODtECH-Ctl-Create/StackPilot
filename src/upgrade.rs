use std::{fs, path::{Path, PathBuf}};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::{scaffold, security, spec::ProjectSpec};

const PROFILE_SCHEMA_VERSION: u64 = 1;
pub const LATEST_GOLDEN_PATH_VERSION: u64 = 2;
const RECIPE_NAME: &str = "base";
const SECURITY_TARGETS: &[(&str, &str)] = &[
    (
        ".github/workflows/security.yml",
        "add the V0.2 dependency, secret, SBOM and conditional container scanning workflow",
    ),
    (
        ".github/dependabot.yml",
        "add the V0.2 ecosystem-aware dependency update policy",
    ),
];
const SECURITY_CONTROLS: &[&str] = &[
    "Dependency update automation",
    "Dependency scanning",
    "Secret scanning",
    "SBOM generation",
    "Container scanning",
    "GitHub Actions permissions",
];

#[derive(Debug, Deserialize)]
struct ProfileDocument {
    version: u64,
    project: ProjectSection,
    features: FeatureSection,
    stackpilot: StackPilotSection,
}

#[derive(Debug, Deserialize)]
struct ProjectSection {
    name: String,
    kind: String,
    language: String,
    framework: String,
    database: String,
    cloud: String,
}

#[derive(Debug, Deserialize)]
struct FeatureSection {
    docker: bool,
    ci: bool,
    terraform: bool,
}

#[derive(Debug, Deserialize)]
struct StackPilotSection {
    recipe: String,
    #[serde(default)]
    golden_path_version: Option<u64>,
}

#[derive(Debug)]
struct PlannedCreate {
    path: PathBuf,
    description: String,
    content: String,
}

#[derive(Debug)]
struct UpgradePlan {
    root: PathBuf,
    current_version: u64,
    target_version: u64,
    metadata_original: String,
    metadata_updated: Option<String>,
    creates: Vec<PlannedCreate>,
}

pub fn run(root: &Path, recipes_dir: &Path, apply: bool) -> Result<()> {
    let plan = plan(root, recipes_dir)?;
    print_plan(&plan, apply);

    if !apply || plan.current_version == plan.target_version {
        return Ok(());
    }

    apply_plan(&plan)?;
    println!(
        "\nApplied StackPilot golden-path upgrade {} -> {}.",
        plan.current_version, plan.target_version
    );
    println!("Application source files were not modified.");
    Ok(())
}

fn plan(root: &Path, recipes_dir: &Path) -> Result<UpgradePlan> {
    if !root.exists() {
        bail!("repository path does not exist: {}", root.display());
    }
    if !root.is_dir() {
        bail!("repository path is not a directory: {}", root.display());
    }

    let root = root
        .canonicalize()
        .with_context(|| format!("failed to resolve repository path {}", root.display()))?;
    let metadata_path = root.join(".stackpilot.toml");
    if !metadata_path.is_file() {
        bail!(
            "{} is not a StackPilot-managed repository: .stackpilot.toml is missing",
            root.display()
        );
    }
    ensure_regular_file(&metadata_path)?;

    let metadata_original = fs::read_to_string(&metadata_path)
        .with_context(|| format!("failed to read {}", metadata_path.display()))?;
    let normalized = normalize_legacy_toml(&metadata_original);
    let profile: ProfileDocument = toml::from_str(&normalized)
        .with_context(|| format!("failed to parse {}", metadata_path.display()))?;

    validate_profile(&profile)?;
    let current_version = profile.stackpilot.golden_path_version.unwrap_or(1);
    if current_version > LATEST_GOLDEN_PATH_VERSION {
        bail!(
            "project golden path v{current_version} is newer than this StackPilot supports (latest v{})",
            LATEST_GOLDEN_PATH_VERSION
        );
    }

    if current_version == LATEST_GOLDEN_PATH_VERSION {
        return Ok(UpgradePlan {
            root,
            current_version,
            target_version: LATEST_GOLDEN_PATH_VERSION,
            metadata_original,
            metadata_updated: None,
            creates: Vec::new(),
        });
    }

    if current_version != 1 {
        bail!(
            "no deterministic StackPilot migration is defined from golden path v{current_version} to v{}",
            LATEST_GOLDEN_PATH_VERSION
        );
    }

    let creates = plan_v1_to_v2_security(&root, recipes_dir, &profile)?;
    let metadata_updated = Some(upgraded_metadata(&normalized)?);

    Ok(UpgradePlan {
        root,
        current_version,
        target_version: LATEST_GOLDEN_PATH_VERSION,
        metadata_original,
        metadata_updated,
        creates,
    })
}

fn plan_v1_to_v2_security(
    root: &Path,
    recipes_dir: &Path,
    profile: &ProfileDocument,
) -> Result<Vec<PlannedCreate>> {
    if !profile.features.ci {
        return Ok(Vec::new());
    }

    let existing = SECURITY_TARGETS
        .iter()
        .filter(|(relative, _)| root.join(relative).is_file())
        .count();

    if existing > 0 {
        let findings = security::inspect(root)?;
        let complete = SECURITY_CONTROLS.iter().all(|name| {
            findings.iter().any(|finding| {
                finding.name == *name && finding.status == crate::inspect::FindingStatus::Passed
            })
        });
        if complete {
            return Ok(Vec::new());
        }

        bail!(
            "existing security automation is not fully compatible with golden path v2; StackPilot will not overwrite custom .github security/dependency files. Run `stackpilot inspect` and align the reported security controls before upgrading"
        );
    }

    if !recipes_dir.is_dir() {
        bail!(
            "StackPilot recipes were not found at {}; pass --recipes-dir or reinstall StackPilot",
            recipes_dir.display()
        );
    }

    let spec = ProjectSpec::configured(
        profile.project.name.clone(),
        profile.project.kind.clone(),
        profile.project.language.clone(),
        profile.project.framework.clone(),
        profile.project.database.clone(),
        profile.project.cloud.clone(),
        profile.features.docker,
        true,
        profile.features.terraform,
    )?;

    let mut creates = Vec::new();
    for (relative, description) in SECURITY_TARGETS {
        let relative = Path::new(relative);
        if let Some(path) = first_symlink_ancestor(root, relative)? {
            bail!(
                "refusing to upgrade because {} contains a symlink at {}",
                relative.display(),
                path.display()
            );
        }
        creates.push(PlannedCreate {
            path: relative.to_path_buf(),
            description: (*description).to_string(),
            content: scaffold::render_destination(&spec, RECIPE_NAME, recipes_dir, relative)?,
        });
    }

    Ok(creates)
}

fn validate_profile(profile: &ProfileDocument) -> Result<()> {
    if profile.version != PROFILE_SCHEMA_VERSION {
        bail!(
            "unsupported .stackpilot.toml schema version {}; this StackPilot supports schema version {}",
            profile.version,
            PROFILE_SCHEMA_VERSION
        );
    }
    if profile.stackpilot.recipe != RECIPE_NAME {
        bail!(
            "golden-path upgrade currently supports recipe '{RECIPE_NAME}', not '{}'",
            profile.stackpilot.recipe
        );
    }
    if !matches!(
        (profile.project.language.as_str(), profile.project.framework.as_str()),
        ("Rust", "Axum")
            | ("Go", "Chi")
            | ("TypeScript", "NestJS")
            | ("Python", "FastAPI")
            | ("Java", "Spring Boot")
            | ("C#", "ASP.NET Core")
    ) {
        bail!(
            "golden-path upgrade requires a supported StackPilot language/framework pair; found {} / {}",
            profile.project.language,
            profile.project.framework
        );
    }
    Ok(())
}

fn upgraded_metadata(normalized: &str) -> Result<String> {
    let mut value: toml::Value = toml::from_str(normalized)?;
    let root = value
        .as_table_mut()
        .context(".stackpilot.toml root must be a TOML table")?;
    let stackpilot = root
        .get_mut("stackpilot")
        .and_then(toml::Value::as_table_mut)
        .context(".stackpilot.toml [stackpilot] section is missing or invalid")?;
    stackpilot.insert(
        "golden_path_version".to_string(),
        toml::Value::Integer(LATEST_GOLDEN_PATH_VERSION as i64),
    );
    let mut output = toml::to_string_pretty(&value)?;
    if !output.ends_with('\n') {
        output.push('\n');
    }
    Ok(output)
}

fn apply_plan(plan: &UpgradePlan) -> Result<()> {
    let metadata_path = plan.root.join(".stackpilot.toml");
    ensure_regular_file(&metadata_path)?;
    let current = fs::read_to_string(&metadata_path)
        .with_context(|| format!("failed to re-read {}", metadata_path.display()))?;
    if current != plan.metadata_original {
        bail!(
            "refusing to apply upgrade because .stackpilot.toml changed after the upgrade plan was created"
        );
    }

    for create in &plan.creates {
        let target = plan.root.join(&create.path);
        if target.exists() {
            bail!(
                "refusing to apply upgrade because {} appeared after planning",
                create.path.display()
            );
        }
        if let Some(path) = first_symlink_ancestor(&plan.root, &create.path)? {
            bail!(
                "refusing to create {} because its target path contains a symlink at {}",
                create.path.display(),
                path.display()
            );
        }
    }

    for create in &plan.creates {
        let target = plan.root.join(&create.path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        fs::write(&target, &create.content)
            .with_context(|| format!("failed to create {}", target.display()))?;
    }

    if let Some(metadata) = &plan.metadata_updated {
        fs::write(&metadata_path, metadata)
            .with_context(|| format!("failed to update {}", metadata_path.display()))?;
    }

    Ok(())
}

fn print_plan(plan: &UpgradePlan, apply: bool) {
    println!("StackPilot golden-path upgrade");
    println!("Repository: {}", plan.root.display());
    println!(
        "Golden path: v{} -> v{}",
        plan.current_version, plan.target_version
    );

    if plan.current_version == plan.target_version {
        println!("Status: already current");
        println!("Application source files: untouched");
        return;
    }

    println!("\nManaged changes");
    for create in &plan.creates {
        println!("  + {} — {}", create.path.display(), create.description);
    }
    if plan.metadata_updated.is_some() {
        println!(
            "  ~ .stackpilot.toml — record golden_path_version = {} and normalize legacy TOML",
            plan.target_version
        );
    }
    println!("\nApplication source files: untouched");
    if !apply {
        println!("Preview only: no files were changed. Re-run with --apply to upgrade.");
    }
}

fn normalize_legacy_toml(raw: &str) -> String {
    raw.replace(" = True", " = true")
        .replace(" = False", " = false")
}

fn ensure_regular_file(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("failed to inspect {}", path.display()))?;
    if metadata.file_type().is_symlink() {
        bail!("refusing to use symlinked managed file {}", path.display());
    }
    if !metadata.is_file() {
        bail!("managed path is not a regular file: {}", path.display());
    }
    Ok(())
}

fn first_symlink_ancestor(root: &Path, relative: &Path) -> Result<Option<PathBuf>> {
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component.as_os_str());
        if current.exists() {
            let metadata = fs::symlink_metadata(&current)
                .with_context(|| format!("failed to inspect {}", current.display()))?;
            if metadata.file_type().is_symlink() {
                return Ok(Some(current));
            }
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{LATEST_GOLDEN_PATH_VERSION, normalize_legacy_toml, upgraded_metadata};

    #[test]
    fn normalizes_legacy_boolean_values() {
        let legacy = "[features]\ndocker = True\nci = False\n";
        let normalized = normalize_legacy_toml(legacy);
        assert!(normalized.contains("docker = true"));
        assert!(normalized.contains("ci = false"));
    }

    #[test]
    fn records_current_golden_path_version() {
        let input = r#"version = 1
[project]
name = "demo"
kind = "Backend API"
language = "Go"
framework = "Chi"
database = "PostgreSQL"
cloud = "AWS"
[features]
docker = true
ci = false
terraform = true
[stackpilot]
recipe = "base"
"#;
        let updated = upgraded_metadata(input).expect("upgrade metadata");
        assert!(updated.contains(&format!(
            "golden_path_version = {}",
            LATEST_GOLDEN_PATH_VERSION
        )));
        let parsed: toml::Value = toml::from_str(&updated).expect("valid TOML");
        assert_eq!(
            parsed["stackpilot"]["golden_path_version"].as_integer(),
            Some(LATEST_GOLDEN_PATH_VERSION as i64)
        );
    }

    #[test]
    fn test_fixture_can_write_stackpilot_metadata() {
        let repo = tempdir().expect("repo");
        fs::write(repo.path().join(".stackpilot.toml"), "version = 1\n").expect("write");
        assert!(repo.path().join(".stackpilot.toml").is_file());
    }
}
