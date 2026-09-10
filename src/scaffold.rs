use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use minijinja::{Environment, context};

use crate::{recipe::Recipe, spec::ProjectSpec};

pub fn create_project(
    spec: &ProjectSpec,
    recipe_name: &str,
    recipes_dir: &Path,
    output_dir: &Path,
) -> Result<PathBuf> {
    let destination = output_dir.join(&spec.name);
    if destination.exists() {
        bail!("destination already exists: {}", destination.display());
    }

    fs::create_dir_all(&destination)
        .with_context(|| format!("failed to create {}", destination.display()))?;

    if let Err(error) = render_in_place(spec, recipe_name, recipes_dir, &destination) {
        let _ = fs::remove_dir_all(&destination);
        return Err(error);
    }

    Ok(destination)
}

pub fn render_in_place(
    spec: &ProjectSpec,
    recipe_name: &str,
    recipes_dir: &Path,
    destination: &Path,
) -> Result<Vec<PathBuf>> {
    let (recipe, recipe_dir) = Recipe::load(recipes_dir, recipe_name)?;
    render_files(spec, &recipe, &recipe_dir, destination)
}

fn render_files(
    spec: &ProjectSpec,
    recipe: &Recipe,
    recipe_dir: &Path,
    destination: &Path,
) -> Result<Vec<PathBuf>> {
    let mut env = Environment::new();
    let mut generated = Vec::new();

    for file in &recipe.files {
        if let Some(condition) = &file.when
            && !matches_condition(condition, spec)?
        {
            continue;
        }

        let source = recipe_dir.join("templates").join(&file.template);
        let template_source = fs::read_to_string(&source)
            .with_context(|| format!("failed to read template {}", source.display()))?;

        env.add_template_owned(file.template.clone(), template_source)
            .with_context(|| format!("failed to load template {}", file.template))?;

        let rendered = env.get_template(&file.template)?.render(context! {
            project_name => spec.name.as_str(),
            project_kind => spec.kind.as_str(),
            language => spec.language.as_str(),
            framework => spec.framework.as_str(),
            database => spec.database.as_str(),
            cloud => spec.cloud.as_str(),
            docker => spec.docker,
            ci => spec.ci,
            terraform => spec.terraform,
            recipe_name => recipe.name.as_str(),
            recipe_description => recipe.description.clone().unwrap_or_default(),
        })?;

        let relative_destination = PathBuf::from(render_destination(&file.destination, &spec.name));
        let target = destination.join(&relative_destination);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&target, rendered)
            .with_context(|| format!("failed to write {}", target.display()))?;
        generated.push(relative_destination);
    }

    Ok(generated)
}

fn matches_condition(condition: &str, spec: &ProjectSpec) -> Result<bool> {
    for clause in condition.split("&&") {
        if !matches_clause(clause.trim(), spec)? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn matches_clause(clause: &str, spec: &ProjectSpec) -> Result<bool> {
    match clause {
        "docker" => Ok(spec.docker),
        "ci" => Ok(spec.ci),
        "terraform" => Ok(spec.terraform),
        "!docker" => Ok(!spec.docker),
        "!ci" => Ok(!spec.ci),
        "!terraform" => Ok(!spec.terraform),
        _ => {
            let Some((key, expected)) = clause.split_once('=') else {
                bail!("unsupported recipe condition: {clause}");
            };

            let actual = match key.trim() {
                "kind" => &spec.kind,
                "language" => &spec.language,
                "framework" => &spec.framework,
                "database" => &spec.database,
                "cloud" => &spec.cloud,
                _ => bail!("unsupported recipe condition key: {}", key.trim()),
            };

            Ok(actual.eq_ignore_ascii_case(expected.trim()))
        }
    }
}

fn render_destination(destination: &str, project_name: &str) -> String {
    destination.replace("{{project_name}}", project_name)
}

#[cfg(test)]
mod tests {
    use super::{matches_condition, render_destination};
    use crate::spec::ProjectSpec;

    #[test]
    fn renders_project_name_in_destination() {
        assert_eq!(
            render_destination("services/{{project_name}}/README.md", "payments"),
            "services/payments/README.md"
        );
    }

    #[test]
    fn evaluates_boolean_and_value_conditions() {
        let spec = ProjectSpec::minimal("payments".to_string()).expect("valid project spec");
        assert!(!matches_condition("docker", &spec).expect("condition should evaluate"));
        assert!(matches_condition("language=Generic", &spec).expect("condition should evaluate"));
    }

    #[test]
    fn evaluates_compound_conditions() {
        let spec = ProjectSpec::configured(
            "payments".to_string(),
            "Backend API".to_string(),
            "Rust".to_string(),
            "Axum".to_string(),
            "PostgreSQL".to_string(),
            "AWS".to_string(),
            true,
            true,
            true,
        )
        .expect("valid project spec");

        assert!(
            matches_condition("language=Rust && framework=Axum && docker", &spec)
                .expect("condition should evaluate")
        );
        assert!(
            !matches_condition("language=Rust && framework=Actix Web", &spec)
                .expect("condition should evaluate")
        );
    }
}
