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
            "Frontend",
            "Full Stack",
            "Worker",
            "CLI",
            "Library",
        ];
        let kind = select(&theme, "Project type", &kinds)?;

        let languages = ["Rust", "Go", "TypeScript", "Python", "Java", "C#"];
        let language = select(&theme, "Language", &languages)?;

        let frameworks = frameworks_for(&language);
        let framework = select(&theme, "Framework", frameworks)?;

        let databases = ["None", "PostgreSQL", "MySQL", "SQLite", "MongoDB"];
        let database = select(&theme, "Database", &databases)?;

        let clouds = ["None", "AWS", "Azure", "GCP"];
        let cloud = select(&theme, "Cloud", &clouds)?;

        let docker = Confirm::with_theme(&theme)
            .with_prompt("Include container foundation?")
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
        validate_non_empty("project type", &kind)?;
        validate_non_empty("language", &language)?;
        validate_non_empty("framework", &framework)?;
        validate_non_empty("database", &database)?;
        validate_non_empty("cloud", &cloud)?;

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

fn frameworks_for(language: &str) -> &'static [&'static str] {
    match language {
        "Rust" => &["Axum", "Actix Web", "Rocket", "None"],
        "Go" => &["Chi", "Gin", "Fiber", "None"],
        "TypeScript" => &["NestJS", "Next.js", "Fastify", "Express", "None"],
        "Python" => &["FastAPI", "Django", "Flask", "None"],
        "Java" => &["Spring Boot", "Quarkus", "Micronaut", "None"],
        "C#" => &["ASP.NET Core", "Worker Service", "Blazor", "None"],
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

fn validate_non_empty(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        bail!("{label} cannot be empty");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ProjectSpec, validate_project_name};

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
    fn creates_configured_non_interactive_spec() {
        let spec = ProjectSpec::configured(
            "worker".to_string(),
            "Worker".to_string(),
            "Go".to_string(),
            "Chi".to_string(),
            "None".to_string(),
            "None".to_string(),
            false,
            true,
            false,
        )
        .expect("valid spec");
        assert_eq!(spec.language, "Go");
        assert!(spec.ci);
    }

    #[test]
    fn rejects_terraform_without_cloud() {
        let result = ProjectSpec::configured(
            "worker".to_string(),
            "Worker".to_string(),
            "Rust".to_string(),
            "None".to_string(),
            "None".to_string(),
            "None".to_string(),
            true,
            true,
            true,
        );
        assert!(result.is_err());
    }
}
