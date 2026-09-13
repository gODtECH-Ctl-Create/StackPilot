use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use minijinja::{Environment, context};

pub const AWS_ECS_FARGATE: &str = "aws-ecs-fargate";

const FOUNDATION_FILES: &[&str] = &[
    "infra/terraform/ecs-fargate.tf",
    "infra/terraform/ecs-fargate-variables.tf",
    "infra/terraform/ecs-fargate-outputs.tf",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeploymentRecommendation {
    pub target: &'static str,
    pub label: &'static str,
    pub reason: String,
}

pub fn normalize_target(target: Option<&str>) -> Result<Option<&'static str>> {
    let Some(target) = target else {
        return Ok(None);
    };
    let normalized = target.trim().to_ascii_lowercase().replace('_', "-");
    match normalized.as_str() {
        "aws-ecs-fargate" | "ecs-fargate" | "fargate" => Ok(Some(AWS_ECS_FARGATE)),
        _ => bail!(
            "unsupported deployment target '{target}'; currently supported: {AWS_ECS_FARGATE}"
        ),
    }
}

pub fn required_cloud(target: &str) -> &'static str {
    match target {
        AWS_ECS_FARGATE => "AWS",
        _ => "None",
    }
}

pub fn recommendation(
    root: &Path,
    languages: &[String],
    frameworks: &[String],
    docker: bool,
) -> Option<DeploymentRecommendation> {
    if !docker || languages.len() != 1 || frameworks.len() != 1 {
        return None;
    }
    if !supported_golden_pair(&languages[0], &frameworks[0]) {
        return None;
    }

    let cloud = metadata_project_value(root, "cloud")?;
    let kind = metadata_project_value(root, "kind")?;
    if !cloud.eq_ignore_ascii_case("AWS") || !matches!(kind.as_str(), "Backend API" | "Worker") {
        return None;
    }

    Some(DeploymentRecommendation {
        target: AWS_ECS_FARGATE,
        label: "AWS ECS/Fargate",
        reason: format!(
            "containerized {} {} with explicit AWS project intent",
            languages[0], frameworks[0]
        ),
    })
}

pub fn foundation_detected(root: &Path, target: &str) -> bool {
    match target {
        AWS_ECS_FARGATE => FOUNDATION_FILES
            .iter()
            .all(|path| root.join(path).is_file()),
        _ => false,
    }
}

pub fn foundation_files(target: &str) -> &'static [&'static str] {
    match target {
        AWS_ECS_FARGATE => FOUNDATION_FILES,
        _ => &[],
    }
}

pub fn existing_terraform_supports_aws(root: &Path) -> Result<bool> {
    let mut files = Vec::new();
    collect_terraform(root, 0, &mut files)?;
    if files.is_empty() {
        return Ok(true);
    }

    for path in files {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read Terraform file {}", path.display()))?;
        let lower = content.to_ascii_lowercase();
        if lower.contains("hashicorp/aws") || lower.contains("provider \"aws\"") {
            return Ok(true);
        }
    }

    Ok(false)
}

pub fn render_foundation_file(
    recipes_dir: &Path,
    target: &str,
    destination: &Path,
    project_name: &str,
) -> Result<String> {
    let template = match (target, destination.to_string_lossy().as_ref()) {
        (AWS_ECS_FARGATE, "infra/terraform/ecs-fargate.tf") => {
            "deployment/aws-ecs-fargate/ecs-fargate.tf.j2"
        }
        (AWS_ECS_FARGATE, "infra/terraform/ecs-fargate-variables.tf") => {
            "deployment/aws-ecs-fargate/variables.tf.j2"
        }
        (AWS_ECS_FARGATE, "infra/terraform/ecs-fargate-outputs.tf") => {
            "deployment/aws-ecs-fargate/outputs.tf.j2"
        }
        _ => bail!(
            "deployment target '{target}' does not define {}",
            destination.display()
        ),
    };

    let source = recipes_dir.join("base/templates").join(template);
    let template_source = fs::read_to_string(&source)
        .with_context(|| format!("failed to read deployment template {}", source.display()))?;
    let service_name = aws_service_name(project_name);

    let mut env = Environment::new();
    env.add_template_owned(template.to_string(), template_source)
        .with_context(|| format!("failed to load deployment template {template}"))?;
    env.get_template(template)?
        .render(context! {
            project_name => project_name,
            service_name => service_name,
        })
        .with_context(|| format!("failed to render deployment template {template}"))
}

fn metadata_project_value(root: &Path, key: &str) -> Option<String> {
    let raw = fs::read_to_string(root.join(".stackpilot.toml")).ok()?;
    // StackPilot v0.1.x rendered template booleans as Python-style True/False.
    // Normalize only assignment values so legacy metadata remains readable while
    // all newly generated files use valid TOML lowercase booleans.
    let normalized = raw
        .replace(" = True", " = true")
        .replace(" = False", " = false");
    let value: toml::Value = toml::from_str(&normalized).ok()?;
    value.get("project")?.get(key)?.as_str().map(str::to_string)
}

fn supported_golden_pair(language: &str, framework: &str) -> bool {
    matches!(
        (language, framework),
        ("Rust", "Axum")
            | ("Go", "Chi")
            | ("TypeScript", "NestJS")
            | ("Python", "FastAPI")
            | ("Java", "Spring Boot")
            | ("C#", "ASP.NET Core")
    )
}

fn aws_service_name(project_name: &str) -> String {
    let mut value = project_name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    while value.contains("--") {
        value = value.replace("--", "-");
    }
    value = value.trim_matches('-').to_string();
    if value.is_empty() {
        value = "stackpilot-service".to_string();
    }
    value.truncate(48);
    value.trim_end_matches('-').to_string()
}

fn collect_terraform(directory: &Path, depth: usize, files: &mut Vec<PathBuf>) -> Result<()> {
    if depth > 5 {
        return Ok(());
    }
    let entries = fs::read_dir(directory)
        .with_context(|| format!("failed to read directory {}", directory.display()))?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            let name = entry.file_name();
            if name.to_str().is_some_and(|name| {
                matches!(
                    name,
                    ".git" | ".terraform" | "node_modules" | "target" | "vendor"
                )
            }) {
                continue;
            }
            collect_terraform(&path, depth + 1, files)?;
        } else if file_type.is_file()
            && path.extension().and_then(|extension| extension.to_str()) == Some("tf")
        {
            files.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{AWS_ECS_FARGATE, aws_service_name, normalize_target, recommendation};

    #[test]
    fn normalizes_supported_deployment_aliases() {
        assert_eq!(
            normalize_target(Some("Fargate")).unwrap(),
            Some(AWS_ECS_FARGATE)
        );
        assert_eq!(
            normalize_target(Some("aws_ecs_fargate")).unwrap(),
            Some(AWS_ECS_FARGATE)
        );
        assert!(normalize_target(Some("kubernetes")).is_err());
    }

    #[test]
    fn sanitizes_aws_service_names() {
        assert_eq!(aws_service_name("Payments_API"), "payments-api");
    }

    #[test]
    fn recommends_ecs_for_containerized_aws_golden_path() {
        let root = tempdir().unwrap();
        fs::write(
            root.path().join(".stackpilot.toml"),
            "[project]\nkind = \"Backend API\"\ncloud = \"AWS\"\n",
        )
        .unwrap();

        let result = recommendation(root.path(), &["Go".to_string()], &["Chi".to_string()], true)
            .expect("deployment recommendation");

        assert_eq!(result.target, AWS_ECS_FARGATE);
        assert_eq!(result.label, "AWS ECS/Fargate");
    }

    #[test]
    fn reads_legacy_metadata_with_uppercase_booleans() {
        let root = tempdir().unwrap();
        fs::write(
            root.path().join(".stackpilot.toml"),
            "version = 1\n\n[project]\nkind = \"Backend API\"\ncloud = \"AWS\"\n\n[features]\ndocker = True\nci = True\nterraform = False\n",
        )
        .unwrap();

        assert!(
            recommendation(root.path(), &["Go".to_string()], &["Chi".to_string()], true).is_some()
        );
    }

    #[test]
    fn does_not_recommend_without_explicit_aws_intent() {
        let root = tempdir().unwrap();
        fs::write(
            root.path().join(".stackpilot.toml"),
            "[project]\nkind = \"Backend API\"\ncloud = \"None\"\n",
        )
        .unwrap();

        assert!(
            recommendation(root.path(), &["Go".to_string()], &["Chi".to_string()], true,).is_none()
        );
    }
}
