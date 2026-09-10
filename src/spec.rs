use anyhow::{Result, bail};
use dialoguer::{Confirm, Input, Select, theme::ColorfulTheme};

#[derive(Debug, Clone)]
pub struct ProjectSpec {
    pub name: String,
    pub kind: String,
    pub language: String,
    pub framework: String,
    pub database: String,
    pub cloud: String,
    pub docker: bool,
    pub ci: bool,
    pub terraform: bool,
}

impl ProjectSpec {
    pub fn interactive(name: Option<String>) -> Result<Self> {
        let theme = ColorfulTheme::default();
        let name = match name {
            Some(name) => name,
            None => Input::<String>::with_theme(&theme)
                .with_prompt("Project name")
                .interact_text()?,
        };
        validate_project_name(&name)?;

        let kinds = [
            "Backend API",
            "Worker",
            "Full Stack",
            "Frontend",
            "CLI",
            "Library",
        ];
        let kind = select(&theme, "Project type", &kinds)?;

        let languages = ["Rust", "Go", "TypeScript", "Python", "Java", "C#"];
        let language = select(&theme, "Language", &languages)?;
        let framework = recommended_framework(&language).to_string();
        println!("StackPilot recommends {language} + {framework}");

        let database = if matches!(kind.as_str(), "Frontend" | "CLI" | "Library") {
            "None".to_string()
        } else {
            let databases = ["PostgreSQL", "None", "MySQL", "MongoDB", "SQLite"];
            select(&theme, "Database", &databases)?
        };

        let cloud = if matches!(kind.as_str(), "CLI" | "Library") {
            "None".to_string()
        } else {
            let clouds = ["AWS", "None", "Azure", "GCP"];
            select(&theme, "Cloud", &clouds)?
        };

        let docker = !matches!(kind.as_str(), "CLI" | "Library")
            && Confirm::with_theme(&theme)
                .with_prompt("Include production container foundation?")
                .default(true)
                .interact()?;

        let ci = Confirm::with_theme(&theme)
            .with_prompt("Include CI foundation?")
            .default(true)
            .interact()?;

        let terraform = if cloud == "None" {
            false
        } else {
            Confirm::with_theme(&theme)
                .with_prompt("Include Terraform foundation?")
                .default(true)
                .interact()?
        };

        Self::configured(
            name, kind, language, framework, database, cloud, docker, ci, terraform,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn configured(
        name: String,
        kind: String,
        language: String,
        framework: String,
        database: String,
        cloud: String,
        docker: bool,
        ci: bool,
        terraform: bool,
    ) -> Result<Self> {
        validate_project_name(&name)?;
        validate_choice(
            "project type",
            &kind,
            &["Backend API", "Worker", "Full Stack", "Frontend", "CLI", "Library", "Generic"],
        )?;
        validate_choice(
            "language",
            &language,
            &["Rust", "Go", "TypeScript", "Python", "Java", "C#", "Generic"],
        )?;
        validate_choice(
            "database",
            &database,
            &["PostgreSQL", "MySQL", "SQLite", "MongoDB", "None"],
        )?;
        validate_choice("cloud", &cloud, &["AWS", "Azure", "GCP", "None"])?;

        let framework = if framework.eq_ignore_ascii_case("Auto") {
            recommended_framework(&language).to_string()
        } else {
            framework
        };

        let supported = supported_frameworks(&language);
        if !supported
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(&framework))
        {
            bail!(
                "framework '{framework}' is not a StackPilot golden path for {language}; choose one of: {}",
                supported.join(", ")
            );
        }

        if terraform && cloud.eq_ignore_ascii_case("None") {
            bail!("Terraform requires a cloud selection");
        }

        Ok(Self {
            name,
            kind,
            language,
            framework,
            database,
            cloud,
            docker,
            ci,
            terraform,
        })
    }
}

fn select(theme: &ColorfulTheme, prompt: &str, items: &[&str]) -> Result<String> {
    let index = Select::with_theme(theme)
        .with_prompt(prompt)
        .items(items)
        .default(0)
        .interact()?;
    Ok(items[index].to_string())
}

fn recommended_framework(language: &str) -> &'static str {
    match language {
        "Rust" => "Axum",
        "Go" => "Chi",
        "TypeScript" => "NestJS",
        "Python" => "FastAPI",
        "Java" => "Spring Boot",
        "C#" => "ASP.NET Core",
        _ => "None",
    }
}

fn supported_frameworks(language: &str) -> &'static [&'static str] {
    match language {
        "Rust" => &["Axum", "None"],
        "Go" => &["Chi", "None"],
        "TypeScript" => &["NestJS", "None"],
        "Python" => &["FastAPI", "None"],
        "Java" => &["Spring Boot", "None"],
        "C#" => &["ASP.NET Core", "None"],
        _ => &["None"],
    }
}

fn validate_project_name(name: &str) -> Result<()> {
    if name.is_empty() || name == "." || name == ".." {
        bail!("project name cannot be empty or a relative path");
    }

    if !name
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        bail!("project name may only contain letters, numbers, '-' and '_'");
    }

    Ok(())
}

fn validate_choice(label: &str, value: &str, allowed: &[&str]) -> Result<()> {
    if !allowed
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(value))
    {
        bail!("unsupported {label} '{value}'; choose one of: {}", allowed.join(", "));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ProjectSpec, recommended_framework, validate_project_name};

    #[test]
    fn accepts_safe_project_names() {
        assert!(validate_project_name("payment-service_2").is_ok());
    }

    #[test]
    fn rejects_path_traversal_and_separators() {
        assert!(validate_project_name("../payment-service").is_err());
        assert!(validate_project_name("payment/service").is_err());
    }

    #[test]
    fn resolves_auto_to_golden_path() {
        let spec = ProjectSpec::configured(
            "worker".to_string(),
            "Worker".to_string(),
            "Go".to_string(),
            "Auto".to_string(),
            "None".to_string(),
            "None".to_string(),
            false,
            true,
            false,
        )
        .expect("valid spec");
        assert_eq!(spec.framework, "Chi");
        assert_eq!(recommended_framework("Python"), "FastAPI");
    }

    #[test]
    fn rejects_non_golden_framework_pair() {
        let result = ProjectSpec::configured(
            "api".to_string(),
            "Backend API".to_string(),
            "Rust".to_string(),
            "Actix Web".to_string(),
            "None".to_string(),
            "None".to_string(),
            false,
            true,
            false,
        );
        assert!(result.is_err());
    }

    #[test]
    fn rejects_terraform_without_cloud() {
        let result = ProjectSpec::configured(
            "worker".to_string(),
            "Worker".to_string(),
            "Rust".to_string(),
            "Auto".to_string(),
            "None".to_string(),
            "None".to_string(),
            true,
            true,
            true,
        );
        assert!(result.is_err());
    }
}
